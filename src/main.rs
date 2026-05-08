mod app;
mod cache;
mod download;
mod feed;
mod search;
mod ui;

use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Result;
use clap::{Parser, Subcommand};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use app::{App, View};

#[derive(Parser)]
#[command(
    name = "saumfar",
    about = "Browse and download Geonorge INSPIRE Atom feeds"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Output directory for downloads
    #[arg(short, long, default_value = ".")]
    output_dir: PathBuf,
}

#[derive(Subcommand)]
enum Command {
    /// List datasets from the service feed
    List {
        /// Fuzzy search filter
        #[arg(short, long)]
        search: Option<String>,
        /// Filter by data owner
        #[arg(long)]
        owner: Option<String>,
    },
    /// Clear the cached service feed
    ClearCache,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = reqwest::Client::builder()
        .user_agent("saumfar-cli/0.1")
        .build()?;

    match cli.command {
        Some(Command::List { search, owner }) => cmd_list(&client, search, owner).await,
        Some(Command::ClearCache) => {
            cache::invalidate().await?;
            println!("Cache cleared.");
            Ok(())
        }
        None => run_tui(&client, &cli.output_dir).await,
    }
}

async fn cmd_list(
    client: &reqwest::Client,
    search: Option<String>,
    owner: Option<String>,
) -> Result<()> {
    let datasets = feed::fetch_service_feed(client).await?;
    let mut fuzzy = search::FuzzySearch::new();

    let titles: Vec<String> = datasets.iter().map(|d| d.title.clone()).collect();
    let indices = match &search {
        Some(q) => fuzzy.filter(q, &titles),
        None => (0..datasets.len()).collect(),
    };

    for i in indices {
        let d = &datasets[i];
        if let Some(ref o) = owner
            && !d.owner.to_lowercase().contains(&o.to_lowercase())
        {
            continue;
        }
        println!(
            "{:<60} {:<25} {:<12} {}",
            d.title,
            d.owner,
            d.crs,
            &d.updated[..10.min(d.updated.len())]
        );
    }

    Ok(())
}

async fn run_tui(client: &reqwest::Client, output_dir: &Path) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    app.loading = true;

    terminal.draw(|f| ui::draw(f, &app))?;

    let datasets = feed::fetch_service_feed(client).await?;
    app.loading = false;
    app.set_datasets(datasets);

    if let Some(age) = cache::cache_age().await {
        let mins = age.as_secs() / 60;
        if mins > 0 {
            app.status_message = Some(format!("Feed cached {mins}m ago"));
        }
    }

    loop {
        terminal.draw(|f| {
            app.visible_rows = f.area().height.saturating_sub(3);
            ui::draw(f, &app);
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    break;
                }

                match app.view {
                    View::Browse => handle_browse_key(&mut app, key, client).await?,
                    View::Detail => {
                        handle_detail_key(&mut app, key, client, output_dir).await?;
                    }
                }

                if app.should_quit {
                    break;
                }
            }
        }

        if let Some(ref dl) = app.downloads {
            let guard = dl.lock().await;
            if guard.iter().all(|d| d.done) {
                let errors: Vec<_> = guard
                    .iter()
                    .filter_map(|d| d.error.as_ref().cloned())
                    .collect();
                let ok_count = guard.len() - errors.len();
                let total_bytes: u64 = guard
                    .iter()
                    .filter(|d| d.error.is_none())
                    .map(|d| d.bytes_downloaded)
                    .sum();
                drop(guard);
                let size = total_bytes as f64 / 1_048_576.0;
                if errors.is_empty() {
                    app.status_message = Some(format!(
                        "Downloaded {ok_count} file{} ({size:.1} MB)",
                        if ok_count == 1 { "" } else { "s" }
                    ));
                } else {
                    app.status_message =
                        Some(format!("{ok_count} downloaded, {} failed", errors.len()));
                }
                app.downloads = None;
            }
        }
    }

    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

async fn handle_browse_key(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    client: &reqwest::Client,
) -> Result<()> {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('s') => app.cycle_sort(),
        KeyCode::Up | KeyCode::Char('k') => app.move_selection(-1),
        KeyCode::Down | KeyCode::Char('j') => app.move_selection(1),
        KeyCode::PageUp => app.page_up(),
        KeyCode::PageDown => app.page_down(),
        KeyCode::Char('g') => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                app.jump_bottom();
            } else {
                app.jump_top();
            }
        }
        KeyCode::Home => app.jump_top(),
        KeyCode::End => app.jump_bottom(),
        KeyCode::Backspace => app.pop_search_char(),
        KeyCode::Esc => app.clear_search(),
        KeyCode::Char('R') => {
            app.loading = true;
            cache::invalidate().await?;
            let datasets = feed::fetch_service_feed(client).await?;
            app.loading = false;
            app.set_datasets(datasets);
            app.status_message = Some("Feed refreshed".to_string());
        }
        KeyCode::Enter => {
            if let Some(ds) = app.selected_dataset() {
                let url = ds.feed_url.clone();
                let title = ds.title.clone();
                if !url.is_empty() {
                    app.view = View::Detail;
                    app.detail_loading = true;
                    app.detail_selected = 0;
                    app.detail_marked.clear();
                    app.current_dataset_title = title;
                    match feed::fetch_dataset_feed(client, &url).await {
                        Ok(entries) => {
                            app.detail_entries = entries;
                            app.detail_loading = false;
                        }
                        Err(e) => {
                            app.status_message = Some(format!("Error: {e}"));
                            app.view = View::Browse;
                            app.detail_loading = false;
                        }
                    }
                }
            }
        }
        KeyCode::Char(c) => app.push_search_char(c),
        _ => {}
    }
    Ok(())
}

async fn handle_detail_key(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    client: &reqwest::Client,
    output_dir: &Path,
) -> Result<()> {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Esc => {
            app.view = View::Browse;
            app.detail_entries.clear();
            app.detail_marked.clear();
        }
        KeyCode::Up | KeyCode::Char('k') => app.move_selection(-1),
        KeyCode::Down | KeyCode::Char('j') => app.move_selection(1),
        KeyCode::PageUp => app.page_up(),
        KeyCode::PageDown => app.page_down(),
        KeyCode::Char('g') => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                app.jump_bottom();
            } else {
                app.jump_top();
            }
        }
        KeyCode::Home => app.jump_top(),
        KeyCode::End => app.jump_bottom(),
        KeyCode::Char(' ') => {
            app.toggle_mark();
            app.move_selection(1);
        }
        KeyCode::Char('d') | KeyCode::Enter => {
            let urls = app.marked_or_selected_urls();
            if !urls.is_empty() && app.downloads.is_none() {
                let count = urls.len();
                app.status_message = Some(format!(
                    "Downloading {count} file{}…",
                    if count == 1 { "" } else { "s" }
                ));
                let state = download::new_shared_downloads();
                app.downloads = Some(state.clone());
                let client = client.clone();
                let dir = output_dir.to_path_buf();
                tokio::spawn(async move {
                    download::download_parallel(&client, urls, &dir, state).await;
                });
            }
        }
        _ => {}
    }
    Ok(())
}

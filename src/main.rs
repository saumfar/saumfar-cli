mod app;
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
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = reqwest::Client::builder()
        .user_agent("saumfar-cli/0.1")
        .build()?;

    match cli.command {
        Some(Command::List { search, owner }) => cmd_list(&client, search, owner).await,
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

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                break;
            }

            match app.view {
                View::Browse => handle_browse_key(&mut app, key.code, client).await?,
                View::Detail => handle_detail_key(&mut app, key.code, client, output_dir).await?,
            }

            if app.should_quit {
                break;
            }
        }
    }

    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

async fn handle_browse_key(app: &mut App, key: KeyCode, client: &reqwest::Client) -> Result<()> {
    match key {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('s') => app.cycle_sort(),
        KeyCode::Up | KeyCode::Char('k') => app.move_selection(-1),
        KeyCode::Down | KeyCode::Char('j') => app.move_selection(1),
        KeyCode::Backspace => app.pop_search_char(),
        KeyCode::Esc => app.clear_search(),
        KeyCode::Enter => {
            if let Some(ds) = app.selected_dataset() {
                let url = ds.feed_url.clone();
                let title = ds.title.clone();
                if !url.is_empty() {
                    app.view = View::Detail;
                    app.detail_loading = true;
                    app.detail_selected = 0;
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
    key: KeyCode,
    client: &reqwest::Client,
    output_dir: &Path,
) -> Result<()> {
    match key {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Esc => {
            app.view = View::Browse;
            app.detail_entries.clear();
        }
        KeyCode::Up | KeyCode::Char('k') => app.move_selection(-1),
        KeyCode::Down | KeyCode::Char('j') => app.move_selection(1),
        KeyCode::Char('d') | KeyCode::Enter => {
            if let Some(entry) = app.selected_download() {
                let url = entry.url.clone();
                app.status_message = Some("Downloading…".to_string());
                match download::download_file(client, &url, output_dir).await {
                    Ok(progress) => {
                        let size = progress.bytes_downloaded as f64 / 1_048_576.0;
                        app.status_message =
                            Some(format!("Downloaded {} ({:.1} MB)", progress.filename, size));
                    }
                    Err(e) => {
                        app.status_message = Some(format!("Download failed: {e}"));
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

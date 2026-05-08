use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Gauge, Paragraph, Row, Table, TableState};

use crate::app::{App, SortColumn, View};
use crate::download::DownloadState;

pub fn draw(f: &mut Frame, app: &App) {
    let has_downloads = app.downloads.is_some();
    let chunks = if has_downloads {
        Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(6),
            Constraint::Length(1),
        ])
        .split(f.area())
    } else {
        Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(0),
            Constraint::Length(1),
        ])
        .split(f.area())
    };

    draw_search_bar(f, app, chunks[0]);

    match app.view {
        View::Browse => draw_browse(f, app, chunks[1]),
        View::Detail => draw_detail(f, app, chunks[1]),
    }

    if has_downloads {
        draw_downloads(f, app, chunks[2]);
    }

    draw_status_bar(f, app, chunks[3]);
}

fn draw_search_bar(f: &mut Frame, app: &App, area: Rect) {
    let search_text = if app.searching {
        Line::from(vec![
            Span::styled(" / ", Style::default().fg(Color::Yellow)),
            Span::raw(&app.search_query),
            Span::styled("▎", Style::default().fg(Color::Yellow)),
        ])
    } else if !app.search_query.is_empty() {
        Line::from(vec![
            Span::styled(" / ", Style::default().fg(Color::DarkGray)),
            Span::styled(&app.search_query, Style::default().fg(Color::DarkGray)),
        ])
    } else {
        Line::from(Span::styled(
            " / search",
            Style::default().fg(Color::DarkGray),
        ))
    };
    f.render_widget(Paragraph::new(search_text), area);
}

fn draw_browse(f: &mut Frame, app: &App, area: Rect) {
    if app.loading {
        let loading =
            Paragraph::new(" Loading service feed…").style(Style::default().fg(Color::DarkGray));
        f.render_widget(loading, area);
        return;
    }

    let sortable = [
        ("Title", SortColumn::Title),
        ("Fmt", SortColumn::Crs), // not sortable, just a placeholder
        ("Owner", SortColumn::Owner),
        ("CRS", SortColumn::Crs),
        ("Updated", SortColumn::Updated),
    ];
    let header_cells = sortable.map(|(label, col)| {
        let is_active = app.sort_column == col && label != "Fmt";
        let style = if is_active {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let arrow = if is_active {
            if app.sort_ascending { " ▲" } else { " ▼" }
        } else {
            ""
        };
        Cell::from(format!("{label}{arrow}")).style(style)
    });
    let header = Row::new(header_cells).height(1).bottom_margin(0);

    let rows: Vec<Row> = app
        .filtered_indices
        .iter()
        .map(|&i| {
            let d = &app.datasets[i];
            Row::new([
                Cell::from(d.title.as_str()),
                Cell::from(d.format.as_str())
                    .style(Style::default().fg(browse_format_color(&d.format))),
                Cell::from(d.owner.as_str()),
                Cell::from(d.crs.as_str()),
                Cell::from(d.updated.get(..10).unwrap_or(&d.updated)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Percentage(40),
        Constraint::Length(10),
        Constraint::Percentage(22),
        Constraint::Percentage(12),
        Constraint::Percentage(14),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::NONE)
                .title(format!(" {} datasets", app.filtered_indices.len())),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(" ▸ ");

    let mut state = TableState::default().with_selected(Some(app.selected));
    f.render_stateful_widget(table, area, &mut state);
}

fn draw_detail(f: &mut Frame, app: &App, area: Rect) {
    if app.detail_loading {
        let loading =
            Paragraph::new(" Loading dataset feed…").style(Style::default().fg(Color::DarkGray));
        f.render_widget(loading, area);
        return;
    }

    let header = Row::new([
        Cell::from("").style(Style::default().fg(Color::DarkGray)),
        Cell::from("Title").style(Style::default().fg(Color::DarkGray)),
        Cell::from("Type").style(Style::default().fg(Color::DarkGray)),
        Cell::from("Format").style(Style::default().fg(Color::DarkGray)),
        Cell::from("Updated").style(Style::default().fg(Color::DarkGray)),
    ])
    .height(1);

    let rows: Vec<Row> = app
        .detail_entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let marker = if app.detail_marked.contains(&i) {
                Cell::from("●").style(Style::default().fg(Color::Cyan))
            } else {
                Cell::from(" ")
            };
            Row::new([
                marker,
                Cell::from(e.title.as_str()),
                Cell::from(e.file_type.as_str())
                    .style(Style::default().fg(type_color(&e.file_type))),
                Cell::from(friendly_format(&e.format)),
                Cell::from(e.updated.get(..10).unwrap_or(&e.updated)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(2),
        Constraint::Percentage(45),
        Constraint::Length(8),
        Constraint::Percentage(22),
        Constraint::Percentage(14),
    ];

    let marked_count = app.detail_marked.len();
    let title = if marked_count > 0 {
        format!(
            " {} — {} entries ({} selected)",
            app.current_dataset_title,
            app.detail_entries.len(),
            marked_count
        )
    } else {
        format!(
            " {} — {} entries",
            app.current_dataset_title,
            app.detail_entries.len()
        )
    };

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::NONE).title(title))
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(" ▸ ");

    let mut state = TableState::default().with_selected(Some(app.detail_selected));
    f.render_stateful_widget(table, area, &mut state);
}

fn draw_downloads(f: &mut Frame, app: &App, area: Rect) {
    let downloads = match &app.downloads {
        Some(d) => d,
        None => return,
    };

    let state = downloads.try_lock();
    let items: Vec<DownloadState> = match state {
        Ok(guard) => guard.clone(),
        Err(_) => return,
    };

    if items.is_empty() {
        return;
    }

    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Downloads");

    let inner = block.inner(area);
    f.render_widget(block, area);

    let row_constraints: Vec<Constraint> = items.iter().map(|_| Constraint::Length(1)).collect();
    let rows = Layout::vertical(row_constraints).split(inner);

    for (i, dl) in items.iter().enumerate() {
        if i >= rows.len() {
            break;
        }

        let cols = Layout::horizontal([Constraint::Length(30), Constraint::Min(0)]).split(rows[i]);

        let label = if dl.done {
            if dl.error.is_some() {
                Span::styled(
                    format!(" ✗ {}", truncate(&dl.filename, 27)),
                    Style::default().fg(Color::Red),
                )
            } else {
                let size = dl.bytes_downloaded as f64 / 1_048_576.0;
                Span::styled(
                    format!(" ✓ {} ({:.1}M)", truncate(&dl.filename, 20), size),
                    Style::default().fg(Color::Green),
                )
            }
        } else {
            Span::styled(
                format!(" ↓ {}", truncate(&dl.filename, 27)),
                Style::default().fg(Color::Yellow),
            )
        };
        f.render_widget(Paragraph::new(label), cols[0]);

        if !dl.done {
            let ratio = match dl.total_bytes {
                Some(total) if total > 0 => dl.bytes_downloaded as f64 / total as f64,
                _ => 0.0,
            };
            let size_text = match dl.total_bytes {
                Some(total) => format!(
                    "{:.1}/{:.1} MB",
                    dl.bytes_downloaded as f64 / 1_048_576.0,
                    total as f64 / 1_048_576.0
                ),
                None => format!("{:.1} MB", dl.bytes_downloaded as f64 / 1_048_576.0),
            };
            let gauge = Gauge::default()
                .ratio(ratio.min(1.0))
                .label(size_text)
                .gauge_style(Style::default().fg(Color::Cyan).bg(Color::Black));
            f.render_widget(gauge, cols[1]);
        } else if let Some(err) = &dl.error {
            f.render_widget(
                Paragraph::new(Span::styled(
                    truncate(err, cols[1].width as usize),
                    Style::default().fg(Color::Red),
                )),
                cols[1],
            );
        }
    }
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let help = if app.searching {
        " type to filter  ↑↓ navigate  ⏎/esc accept  ⌥⌫ clear"
    } else {
        match app.view {
            View::Browse => {
                " / search  s sort  j/k navigate  g/G top/bottom  ⏎ open  R refresh  q quit"
            }
            View::Detail => {
                " space mark  d download  j/k navigate  g/G top/bottom  esc back  q quit"
            }
        }
    };

    let line = if let Some(msg) = &app.status_message {
        Line::from(vec![
            Span::styled(format!(" {msg} "), Style::default().fg(Color::Green)),
            Span::styled(help, Style::default().fg(Color::DarkGray)),
        ])
    } else {
        Line::from(Span::styled(help, Style::default().fg(Color::DarkGray)))
    };

    f.render_widget(
        Paragraph::new(line).style(Style::default().bg(Color::Black).bold()),
        area,
    );
}

fn browse_format_color(fmt: &str) -> Color {
    match fmt {
        "GML" => Color::Cyan,
        "SOSI" => Color::Magenta,
        "GeoJSON" => Color::Yellow,
        "FGDB" | "GeoPackage" | "PostGIS" => Color::Blue,
        "Shape" => Color::Green,
        "TIFF" => Color::Red,
        "PDF" => Color::Red,
        "CSV" | "Excel" => Color::White,
        _ => Color::DarkGray,
    }
}

fn type_color(file_type: &str) -> Color {
    match file_type {
        "gml" => Color::Cyan,
        "sosi" => Color::Magenta,
        "json" | "geojson" => Color::Yellow,
        "zip" => Color::Green,
        "gpkg" | "fgdb" | "gdb" | "sql" => Color::Blue,
        "tif" | "tiff" | "pdf" => Color::Red,
        "csv" | "xlsx" => Color::White,
        _ => Color::DarkGray,
    }
}

fn friendly_format(mime: &str) -> &str {
    if mime.contains("atom") {
        "Atom feed"
    } else if mime.contains("gml") {
        "GML"
    } else if mime.contains("json") {
        "JSON"
    } else if mime.contains("xml") {
        "XML"
    } else if mime.contains("zip") {
        "ZIP"
    } else if mime.contains("octet-stream") {
        "binary"
    } else if mime.is_empty() {
        ""
    } else {
        mime
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}

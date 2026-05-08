use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};

use crate::app::{App, SortColumn, View};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(1), // search bar
        Constraint::Min(0),    // main content
        Constraint::Length(1), // status bar
    ])
    .split(f.area());

    draw_search_bar(f, app, chunks[0]);

    match app.view {
        View::Browse => draw_browse(f, app, chunks[1]),
        View::Detail => draw_detail(f, app, chunks[1]),
    }

    draw_status_bar(f, app, chunks[2]);
}

fn draw_search_bar(f: &mut Frame, app: &App, area: Rect) {
    let search_text = if app.search_query.is_empty() {
        Line::from(vec![
            Span::styled(" / ", Style::default().fg(Color::DarkGray)),
            Span::styled("search datasets…", Style::default().fg(Color::DarkGray)),
        ])
    } else {
        Line::from(vec![
            Span::styled(" / ", Style::default().fg(Color::Yellow)),
            Span::raw(&app.search_query),
            Span::styled("▎", Style::default().fg(Color::Yellow)),
        ])
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

    let header_cells = [
        ("Title", SortColumn::Title),
        ("Owner", SortColumn::Owner),
        ("CRS", SortColumn::Crs),
        ("Updated", SortColumn::Updated),
    ]
    .map(|(label, col)| {
        let style = if app.sort_column == col {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let arrow = if app.sort_column == col {
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
                Cell::from(d.owner.as_str()),
                Cell::from(d.crs.as_str()),
                Cell::from(d.updated.get(..10).unwrap_or(&d.updated)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Percentage(45),
        Constraint::Percentage(25),
        Constraint::Percentage(12),
        Constraint::Percentage(18),
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
        Cell::from("Title").style(Style::default().fg(Color::DarkGray)),
        Cell::from("Format").style(Style::default().fg(Color::DarkGray)),
        Cell::from("Updated").style(Style::default().fg(Color::DarkGray)),
    ])
    .height(1);

    let rows: Vec<Row> = app
        .detail_entries
        .iter()
        .map(|e| {
            Row::new([
                Cell::from(e.title.as_str()),
                Cell::from(e.format.as_str()),
                Cell::from(e.updated.get(..10).unwrap_or(&e.updated)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Percentage(55),
        Constraint::Percentage(25),
        Constraint::Percentage(20),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::NONE).title(format!(
            " {} — {} entries",
            app.current_dataset_title,
            app.detail_entries.len()
        )))
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(" ▸ ");

    let mut state = TableState::default().with_selected(Some(app.detail_selected));
    f.render_stateful_widget(table, area, &mut state);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let help = match app.view {
        View::Browse => " / search  s sort  ↑↓ navigate  ⏎ open  q quit",
        View::Detail => " d download  ↑↓ navigate  esc back  q quit",
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

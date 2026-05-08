use std::collections::HashSet;

use crate::download::SharedDownloads;
use crate::feed::{Dataset, DownloadEntry};
use crate::search::FuzzySearch;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum View {
    Browse,
    Detail,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortColumn {
    Title,
    Owner,
    Crs,
    Updated,
}

impl SortColumn {
    pub fn next(self) -> Self {
        match self {
            Self::Title => Self::Owner,
            Self::Owner => Self::Crs,
            Self::Crs => Self::Updated,
            Self::Updated => Self::Title,
        }
    }
}

pub struct App {
    pub view: View,
    pub datasets: Vec<Dataset>,
    pub filtered_indices: Vec<usize>,
    pub selected: usize,
    pub search_query: String,
    pub sort_column: SortColumn,
    pub sort_ascending: bool,
    pub search: FuzzySearch,
    pub visible_rows: u16,

    // Detail view
    pub detail_entries: Vec<DownloadEntry>,
    pub detail_selected: usize,
    pub detail_loading: bool,
    pub current_dataset_title: String,
    pub detail_marked: HashSet<usize>,

    // Downloads
    pub downloads: Option<SharedDownloads>,

    // Status
    pub status_message: Option<String>,
    pub loading: bool,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            view: View::Browse,
            datasets: Vec::new(),
            filtered_indices: Vec::new(),
            selected: 0,
            search_query: String::new(),
            sort_column: SortColumn::Title,
            sort_ascending: true,
            search: FuzzySearch::new(),
            visible_rows: 20,

            detail_entries: Vec::new(),
            detail_selected: 0,
            detail_loading: false,
            current_dataset_title: String::new(),
            detail_marked: HashSet::new(),

            downloads: None,

            status_message: None,
            loading: false,
            should_quit: false,
        }
    }

    pub fn set_datasets(&mut self, datasets: Vec<Dataset>) {
        self.datasets = datasets;
        self.sort_datasets();
        self.apply_filter();
    }

    pub fn apply_filter(&mut self) {
        let titles: Vec<String> = self.datasets.iter().map(|d| d.title.clone()).collect();
        self.filtered_indices = self.search.filter(&self.search_query, &titles);
        self.selected = 0;
    }

    pub fn sort_datasets(&mut self) {
        let asc = self.sort_ascending;
        self.datasets.sort_by(|a, b| {
            let cmp = match self.sort_column {
                SortColumn::Title => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
                SortColumn::Owner => a.owner.to_lowercase().cmp(&b.owner.to_lowercase()),
                SortColumn::Crs => a.crs.cmp(&b.crs),
                SortColumn::Updated => a.updated.cmp(&b.updated),
            };
            if asc { cmp } else { cmp.reverse() }
        });
    }

    pub fn cycle_sort(&mut self) {
        self.sort_column = self.sort_column.next();
        self.sort_datasets();
        self.apply_filter();
    }

    fn list_len(&self) -> usize {
        match self.view {
            View::Browse => self.filtered_indices.len(),
            View::Detail => self.detail_entries.len(),
        }
    }

    fn selected_mut(&mut self) -> &mut usize {
        match self.view {
            View::Browse => &mut self.selected,
            View::Detail => &mut self.detail_selected,
        }
    }

    pub fn move_selection(&mut self, delta: i32) {
        let len = self.list_len();
        if len == 0 {
            return;
        }
        let sel = self.selected_mut();
        *sel = (*sel as i32 + delta).rem_euclid(len as i32) as usize;
    }

    pub fn page_up(&mut self) {
        let page = self.visible_rows.saturating_sub(2) as usize;
        let len = self.list_len();
        if len == 0 {
            return;
        }
        let sel = self.selected_mut();
        *sel = sel.saturating_sub(page);
    }

    pub fn page_down(&mut self) {
        let page = self.visible_rows.saturating_sub(2) as usize;
        let len = self.list_len();
        if len == 0 {
            return;
        }
        let sel = self.selected_mut();
        *sel = (*sel + page).min(len - 1);
    }

    pub fn jump_top(&mut self) {
        *self.selected_mut() = 0;
    }

    pub fn jump_bottom(&mut self) {
        let len = self.list_len();
        if len == 0 {
            return;
        }
        *self.selected_mut() = len - 1;
    }

    pub fn selected_dataset(&self) -> Option<&Dataset> {
        self.filtered_indices
            .get(self.selected)
            .and_then(|&i| self.datasets.get(i))
    }

    pub fn selected_download(&self) -> Option<&DownloadEntry> {
        self.detail_entries.get(self.detail_selected)
    }

    pub fn toggle_mark(&mut self) {
        if self.view != View::Detail {
            return;
        }
        if self.detail_marked.contains(&self.detail_selected) {
            self.detail_marked.remove(&self.detail_selected);
        } else {
            self.detail_marked.insert(self.detail_selected);
        }
    }

    pub fn marked_or_selected_urls(&self) -> Vec<String> {
        if self.detail_marked.is_empty() {
            self.selected_download()
                .map(|e| vec![e.url.clone()])
                .unwrap_or_default()
        } else {
            self.detail_marked
                .iter()
                .filter_map(|&i| self.detail_entries.get(i))
                .map(|e| e.url.clone())
                .collect()
        }
    }

    pub fn push_search_char(&mut self, c: char) {
        self.search_query.push(c);
        self.apply_filter();
    }

    pub fn pop_search_char(&mut self) {
        self.search_query.pop();
        self.apply_filter();
    }

    pub fn clear_search(&mut self) {
        self.search_query.clear();
        self.apply_filter();
    }
}

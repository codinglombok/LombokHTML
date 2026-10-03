//! Table extraction (SPEC section 10).

use crate::dom::{Document, NodeId, NodeKind};
use crate::meta::{collapse, is_ws};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// One table: rows of cell texts, spans expanded, gaps filled with `""`.
pub type Table = Vec<Vec<String>>;

/// Most cells produced for one table; extraction stops there.
pub const MAX_CELLS: usize = 1_000_000;

/// Leading-integer span attribute, 0 or invalid giving `default`, capped at `maximum`.
fn parse_span(v: Option<&str>, default: usize, maximum: usize) -> usize {
    let Some(v) = v else { return default };
    let t = v.trim_start_matches(is_ws);
    let t = t.strip_prefix('+').unwrap_or(t);
    let end = t.bytes().take_while(u8::is_ascii_digit).count();
    if end == 0 {
        return default;
    }
    let n = if end <= 9 {
        t[..end].parse::<usize>().unwrap_or(maximum)
    } else {
        maximum
    };
    if n == 0 {
        return default;
    }
    n.min(maximum)
}

impl Document {
    fn collect_rows(&self, id: NodeId, rows: &mut Vec<NodeId>) {
        for &c in &self.node(id).children {
            let n = self.node(c);
            if n.kind != NodeKind::Element || n.name == "table" {
                continue;
            }
            if n.name == "tr" {
                rows.push(c);
            } else {
                self.collect_rows(c, rows);
            }
        }
    }

    /// Every `table` element in document order, as a grid (SPEC section 10).
    pub fn tables(&self) -> Vec<Table> {
        let mut tables = Vec::new();
        for tid in self.elements() {
            if !self.node(tid).is("table") {
                continue;
            }
            let mut rows = Vec::new();
            self.collect_rows(tid, &mut rows);
            let mut grid: Table = Vec::new();
            let mut pending: BTreeMap<usize, (usize, String)> = BTreeMap::new();
            let mut total = 0usize;
            let mut stop = false;
            for tr in rows {
                let mut row: BTreeMap<usize, String> = BTreeMap::new();
                let mut next: BTreeMap<usize, (usize, String)> = BTreeMap::new();
                for (col, (left, text)) in &pending {
                    row.insert(*col, text.clone());
                    if *left > 1 {
                        next.insert(*col, (left - 1, text.clone()));
                    }
                }
                let mut col = 0usize;
                for &cid in &self.node(tr).children {
                    let cell = self.node(cid);
                    if !(cell.is("td") || cell.is("th")) {
                        continue;
                    }
                    let text = collapse(&self.text_content(cid));
                    let cs = parse_span(cell.attr("colspan"), 1, 1000);
                    let rs = parse_span(cell.attr("rowspan"), 1, 65534);
                    for _ in 0..cs {
                        while row.contains_key(&col) {
                            col += 1;
                        }
                        total += 1;
                        if total > MAX_CELLS {
                            stop = true;
                            break;
                        }
                        row.insert(col, text.clone());
                        if rs > 1 {
                            next.insert(col, (rs - 1, text.clone()));
                        }
                        col += 1;
                    }
                    if stop {
                        break;
                    }
                }
                pending = next;
                let width = row.keys().next_back().map_or(0, |m| m + 1);
                grid.push(
                    (0..width)
                        .map(|c| row.get(&c).cloned().unwrap_or_default())
                        .collect(),
                );
                if stop {
                    break;
                }
            }
            tables.push(grid);
        }
        tables
    }
}

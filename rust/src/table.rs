//! Table extraction: each `<table>` becomes rows of cell text (whitespace
//! collapsed). `colspan` repeats the text across columns and `rowspan` carries
//! it down into following rows so every row aligns with the visual grid.
//! Span values are clamped (colspan ≤ 1000, rowspan ≤ 65534, per the HTML
//! spec) so hostile input cannot force huge allocations. Rows of *nested*
//! tables belong to the nested table only.

use crate::dom::Node;
use alloc::string::String;
use alloc::vec::Vec;

pub type Row = Vec<String>;
pub type Table = Vec<Row>;

const MAX_COLSPAN: usize = 1000;
const MAX_ROWSPAN: usize = 65534;
const MAX_CELLS: usize = 1_000_000;

fn cell_text(cell: &Node) -> String {
    fn collect(node: &Node, out: &mut String) {
        if let Some(t) = &node.text {
            out.push_str(t);
            out.push(' ');
        }
        for c in &node.children {
            collect(c, out);
        }
    }
    let mut out = String::new();
    for c in &cell.children {
        collect(c, &mut out);
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn span(cell: &Node, attr: &str, max: usize) -> usize {
    cell.attr(attr)
        .and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|n| *n > 0)
        .map(|n| n.min(max))
        .unwrap_or(1)
}

fn collect_rows<'a>(node: &'a Node, rows: &mut Vec<&'a Node>) {
    for c in &node.children {
        if c.is_element("table") {
            continue; // nested table: its rows are its own
        }
        if c.is_element("tr") {
            rows.push(c);
        } else {
            collect_rows(c, rows);
        }
    }
}

/// Extract every `<table>` (including nested ones) in `doc`.
pub fn extract_tables(doc: &Node) -> Vec<Table> {
    doc.find_all("table")
        .into_iter()
        .map(extract_one_table)
        .collect()
}

fn extract_one_table(table: &Node) -> Table {
    let mut trs = Vec::new();
    collect_rows(table, &mut trs);
    let mut grid: Table = Vec::new();
    // pending[col] = (rows_left, text) carried from a rowspan above.
    let mut pending: Vec<Option<(usize, String)>> = Vec::new();
    let mut total = 0usize;

    for tr in trs {
        let mut row: Row = Vec::new();
        let mut col = 0usize;
        let fill_pending =
            |row: &mut Row, col: &mut usize, pending: &mut Vec<Option<(usize, String)>>| {
                while let Some(Some((left, text))) = pending.get_mut(*col) {
                    row.push(text.clone());
                    *left -= 1;
                    if *left == 0 {
                        pending[*col] = None;
                    }
                    *col += 1;
                }
            };
        for cell in tr
            .children
            .iter()
            .filter(|c| c.is_element("td") || c.is_element("th"))
        {
            fill_pending(&mut row, &mut col, &mut pending);
            let text = cell_text(cell);
            let cs = span(cell, "colspan", MAX_COLSPAN);
            let rs = span(cell, "rowspan", MAX_ROWSPAN);
            for _ in 0..cs {
                total += 1;
                if total > MAX_CELLS {
                    return grid;
                }
                row.push(text.clone());
                if rs > 1 {
                    if pending.len() <= col {
                        pending.resize(col + 1, None);
                    }
                    pending[col] = Some((rs - 1, text.clone()));
                }
                col += 1;
            }
        }
        fill_pending(&mut row, &mut col, &mut pending);
        // Trailing columns that are only continued by rowspans from above.
        loop {
            if col >= pending.len() {
                break;
            }
            if pending[col].is_some() {
                fill_pending(&mut row, &mut col, &mut pending);
            } else {
                break;
            }
        }
        grid.push(row);
    }
    grid
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::parse;

    fn t(html: &str) -> Vec<Table> {
        extract_tables(&parse(html))
    }

    #[test]
    fn simple_table() {
        let r = t(
            "<table><tr><th>Name</th><th>Age</th></tr><tr><td>Alice</td><td>30</td></tr></table>",
        );
        assert_eq!(r[0][0], ["Name", "Age"]);
        assert_eq!(r[0][1], ["Alice", "30"]);
    }

    #[test]
    fn thead_tbody_and_multiple_tables() {
        let r = t("<table><thead><tr><th>A</th></tr></thead><tbody><tr><td>1</td></tr></tbody></table><table><tr><td>2</td></tr></table>");
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].len(), 2);
    }

    #[test]
    fn colspan_repeats() {
        let r = t(r#"<table><tr><td colspan="2">M</td><td>C</td></tr></table>"#);
        assert_eq!(r[0][0], ["M", "M", "C"]);
    }

    #[test]
    fn rowspan_carries_down() {
        let r = t(r#"<table><tr><td rowspan="2">A</td><td>B</td></tr><tr><td>C</td></tr></table>"#);
        assert_eq!(r[0][0], ["A", "B"]);
        assert_eq!(r[0][1], ["A", "C"]);
    }

    #[test]
    fn rowspan_in_middle_column_and_trailing() {
        let r = t(
            r#"<table><tr><td>1</td><td rowspan="2">X</td><td>3</td></tr><tr><td>a</td><td>c</td></tr></table>"#,
        );
        assert_eq!(r[0][1], ["a", "X", "c"]);
        let r2 =
            t(r#"<table><tr><td>1</td><td rowspan="2">X</td></tr><tr><td>a</td></tr></table>"#);
        assert_eq!(r2[0][1], ["a", "X"]);
    }

    #[test]
    fn colspan_and_rowspan_together() {
        let r = t(
            r#"<table><tr><td colspan="2" rowspan="2">Z</td><td>1</td></tr><tr><td>2</td></tr></table>"#,
        );
        assert_eq!(r[0][0], ["Z", "Z", "1"]);
        assert_eq!(r[0][1], ["Z", "Z", "2"]);
    }

    #[test]
    fn hostile_spans_are_clamped() {
        let r = t(r#"<table><tr><td colspan="4000000000">x</td></tr></table>"#);
        assert_eq!(r[0][0].len(), MAX_COLSPAN);
        let r = t(r#"<table><tr><td rowspan="99999999999">x</td></tr></table>"#);
        assert_eq!(r[0][0], ["x"]);
    }

    #[test]
    fn nested_table_rows_do_not_leak_into_outer() {
        let r = t("<table><tr><td>outer<table><tr><td>inner</td></tr></table></td></tr></table>");
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].len(), 1);
        assert_eq!(r[1][0], ["inner"]);
    }

    #[test]
    fn nested_markup_collapses() {
        let r = t("<table><tr><td>Hello <b>world</b></td></tr></table>");
        assert_eq!(r[0][0][0], "Hello world");
    }
}

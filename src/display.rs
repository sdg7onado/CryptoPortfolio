use crate::portfolio::Portfolio;
use comfy_table::{Cell, Color, Table};

pub fn display_portfolio(portfolio: &Portfolio, total_value: f64) {
    let mut table = Table::new();
    table.set_header(vec![
        "Symbol",
        "Quantity",
        "Purchase Price",
        "Stop-Loss",
        "Current Value",
    ]);

    for holding in &portfolio.holdings {
        let current_value = holding.quantity * holding.purchase_price; // Placeholder; update with real price
        table.add_row(vec![
            Cell::new(&holding.symbol).fg(Color::Green),
            Cell::new(format!("{:.2}", holding.quantity)),
            Cell::new(format!("${:.2}", holding.purchase_price)),
            Cell::new(format!("${:.2}", holding.stop_loss)),
            Cell::new(format!("${:.2}", current_value)).fg(Color::Cyan),
        ]);
    }

    table.add_row(vec![
        Cell::new("Cash").fg(Color::Yellow),
        Cell::new(format!("${:.2}", portfolio.cash)),
        Cell::new(""),
        Cell::new(""),
        Cell::new(""),
    ]);

    table.add_row(vec![
        Cell::new("Total").fg(Color::White),
        Cell::new(""),
        Cell::new(""),
        Cell::new(""),
        Cell::new(format!("${:.2}", total_value)).fg(Color::White),
    ]);

    println!("\n=== Portfolio Status ===");
    println!("{}", table);
}

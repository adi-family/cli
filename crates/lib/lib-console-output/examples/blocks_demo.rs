use lib_console_output::blocks::{Card, Columns, KeyValue, List, Renderable, Section, Table};
use lib_console_output::theme;

fn main() {
    // Section
    println!();
    print!("{}", Section::new("Table (bordered)").width(50));

    // Table
    let table = Table::new()
        .header(["Service", "Status", "Port", "Health"])
        .row([
            "web-frontend",
            &theme::success("running").to_string(),
            "8080",
            &theme::success("healthy").to_string(),
        ])
        .row([
            "api-server",
            &theme::success("running").to_string(),
            "3000",
            &theme::success("healthy").to_string(),
        ])
        .row([
            "worker",
            &theme::error("stopped").to_string(),
            "—",
            &theme::error("down").to_string(),
        ])
        .row([
            "scheduler",
            &theme::warning("starting").to_string(),
            "9090",
            &theme::muted("pending").to_string(),
        ]);
    print!("{}", table);

    // Section + Columns
    println!();
    print!("{}", Section::new("Columns (borderless)").width(50));

    let cols = Columns::new()
        .header(["NAME", "TYPE", "SERVICES"])
        .row(["default", "directory", "4"])
        .row(["plugins", "registry", "12"])
        .row(["external", "git", "2"]);
    print!("{}", cols);

    // Section + Card
    println!();
    print!("{}", Section::new("Card").width(50));

    let card = Card::new()
        .title("Daemon Status")
        .line(&format!(
            "PID:       {}",
            theme::brand("1234")
        ))
        .line(&format!(
            "Version:   {}",
            theme::brand("0.5.0")
        ))
        .line(&format!(
            "Uptime:    {}",
            theme::muted("3600s")
        ))
        .line(&format!(
            "Services:  {} / {}",
            theme::success("4"),
            "5"
        ));
    print!("{}", card);

    // Section + KeyValue
    println!();
    print!("{}", Section::new("KeyValue").width(50));

    let kv = KeyValue::new()
        .entry("PID", "1234")
        .entry("Version", "0.5.0")
        .entry("Uptime", "3600s")
        .entry("Running services", &format!("{}/{}", theme::success("4"), "5"))
        .entry("Proxy", &theme::brand("http://adi.local").to_string());
    print!("{}", kv);

    // Section + List (bullets)
    println!();
    print!("{}", Section::new("List (bullets)").width(50));

    let list = List::new()
        .item("web-frontend started on :8080")
        .item("api-server started on :3000")
        .item("worker failed to start")
        .item("scheduler is starting...");
    print!("{}", list);

    // Section + List (numbered)
    println!();
    print!("{}", Section::new("List (numbered)").width(50));

    let numbered = List::new()
        .item("Initialize database")
        .item("Start auth service")
        .item("Start API server")
        .item("Start web frontend")
        .numbered(true);
    print!("{}", numbered);

    // Line count demo
    println!();
    print!("{}", Section::new("Line counts").width(50));
    println!("  Table:    {} lines", table.line_count());
    println!("  Columns:  {} lines", cols.line_count());
    println!("  Card:     {} lines", card.line_count());
    println!("  KeyValue: {} lines", kv.line_count());
    println!("  List:     {} lines", list.line_count());
    println!("  Section:  {} lines", Section::new("x").line_count());
}

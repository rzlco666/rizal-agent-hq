mod cli;

#[tokio::main]
async fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        println!("rizal-agent-hq {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    let status = cli::run(&args).await;
    if status != 0 {
        std::process::exit(status);
    }
}

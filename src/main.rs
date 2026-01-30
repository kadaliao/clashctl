use anyhow::Result;
use clap::Parser;

mod app;
mod clash;
mod config;
mod ui;

#[derive(Parser)]
#[command(name = "clashctl")]
#[command(version = "0.1.0")]
#[command(about = "A simple-first TUI Clash controller", long_about = None)]
struct Cli {
    /// Clash External Controller API URL
    #[arg(long, default_value = "http://127.0.0.1:9090")]
    api_url: String,

    /// Clash External Controller secret
    #[arg(long)]
    secret: Option<String>,

    /// Test API connection and print status
    #[arg(long)]
    test: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load or create config
    let mut config = config::AppConfig::load().unwrap_or_default();

    // Merge CLI arguments into config
    let api_url = if cli.api_url != "http://127.0.0.1:9090" {
        Some(cli.api_url.clone())
    } else {
        None
    };
    config.merge_cli(api_url, cli.secret.clone());

    // Save config for next time
    let _ = config.save();

    // Get preset
    let preset = config::Preset::from_str(&config.current_preset).unwrap_or_default();

    // Test mode - just test connection and print info
    if cli.test {
        return test_api_connection(&config.api_url, &config.secret).await;
    }

    // Start TUI
    ui::run(
        config.api_url.clone(),
        config.secret.clone(),
        preset,
        &mut config,
    )
    .await?;

    Ok(())
}

async fn test_api_connection(api_url: &str, secret: &Option<String>) -> Result<()> {
    use clash::ClashClient;

    println!("Testing connection to Clash API at {}...", api_url);

    let client = ClashClient::new(api_url.to_string(), secret.clone());

    // Test connection
    match client.test_connection().await {
        Ok(_) => println!("✓ Connected successfully!"),
        Err(e) => {
            eprintln!("✗ Connection failed: {}", e);
            std::process::exit(1);
        }
    }

    // Get config
    println!("\nFetching configuration...");
    match client.get_config().await {
        Ok(config) => {
            println!("✓ Configuration:");
            println!(
                "  Mode: {}",
                config
                    .mode
                    .as_ref()
                    .map(|mode| format!("{:?}", mode))
                    .unwrap_or_else(|| "Unknown".to_string())
            );
            println!("  HTTP Port: {}", config.port);
            println!("  SOCKS Port: {}", config.socks_port);
            println!("  Allow LAN: {}", config.allow_lan);
        }
        Err(e) => {
            eprintln!("✗ Failed to get config: {}", e);
        }
    }

    // Get proxies
    println!("\nFetching proxy groups...");
    match client.get_proxies().await {
        Ok(proxies) => {
            println!("✓ Found {} proxy groups:", proxies.proxies.len());
            let mut proxy_list: Vec<_> = proxies.proxies.iter().collect();
            proxy_list.sort_by_key(|(name, _)| *name);

            for (name, proxy) in proxy_list.iter().take(10) {
                println!("  - {} ({:?})", name, proxy.proxy_type);
                if let Some(now) = &proxy.now {
                    println!("    Current: {}", now);
                }
                if let Some(all) = &proxy.all {
                    println!("    Options: {} nodes", all.len());
                }
            }

            if proxy_list.len() > 10 {
                println!("  ... and {} more", proxy_list.len() - 10);
            }
        }
        Err(e) => {
            eprintln!("✗ Failed to get proxies: {}", e);
        }
    }

    // Get rules
    println!("\nFetching rules...");
    match client.get_rules().await {
        Ok(rules) => {
            println!("✓ Found {} rules", rules.rules.len());
            for rule in rules.rules.iter().take(5) {
                println!("  - {} {} -> {}", rule.rule_type, rule.payload, rule.proxy);
            }
            if rules.rules.len() > 5 {
                println!("  ... and {} more", rules.rules.len() - 5);
            }
        }
        Err(e) => {
            eprintln!("✗ Failed to get rules: {}", e);
        }
    }

    println!("\n✓ All tests completed successfully!");

    Ok(())
}

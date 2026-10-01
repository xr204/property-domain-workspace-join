use property_domain_join::infrai_client::InfraiClient;
use property_domain_join::property_access::{join_employee, prove_domain};

fn required(args: &mut impl Iterator<Item = String>, label: &str) -> Result<String, String> {
    args.next().ok_or_else(|| format!("missing {label}"))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let command = required(&mut args, "command")?;
    let client = InfraiClient::from_env()?;

    match command.as_str() {
        "prove-domain" => {
            let domain = required(&mut args, "domain")?;
            let proof_name = required(&mut args, "proof name")?;
            let proof_content = required(&mut args, "proof content")?;
            let zone_id = prove_domain(&client, &domain, &proof_name, &proof_content).await?;
            println!("domain proof record is ready in zone {zone_id}");
        }
        "join-employee" => {
            let domain = required(&mut args, "domain")?;
            let email = required(&mut args, "email")?;
            let name = required(&mut args, "name")?;
            let result = join_employee(&client, &domain, &email, &name).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        _ => return Err(format!("unknown command: {command}").into()),
    }
    Ok(())
}

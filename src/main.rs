mod config;

use std::{
    io::{Read, Write},
    net::TcpStream,
    path::Path,
};

use clap::Parser;
use config::{read_file, DoDockerConfig};
use do_sdk::client::{
    droplets::DropletHelper,
    model::{CreateOneDropletReq, Droplet},
    DoClient,
};
use ssh2::Session;

#[derive(Debug, Parser)]
struct Args {
    #[clap(short, long)]
    config: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config = DoDockerConfig::read_from_file(&args.config)?;
    operator_do(config).await?;
    Ok(())
}

async fn operator_do(config: DoDockerConfig) -> anyhow::Result<()> {
    let droplet = create_drop(&config).await?;
    let id = droplet.id;
    let ip = droplet
        .networks
        .v4
        .into_iter()
        .filter_map(|v| (v.r#type == "public").then_some(v.ip_address))
        .next()
        .expect("should be with public ip");
    println!("get ip: {}", ip);
    tokio::time::sleep(std::time::Duration::from_secs(30)).await;

    do_session(ip, &config).await?;

    println!("sesson over will delete droplet in 300s");

    tokio::time::sleep(std::time::Duration::from_secs(300)).await;

    delete_drop(&config, id).await?;

    Ok(())
}

async fn create_drop(config: &DoDockerConfig) -> anyhow::Result<Droplet> {
    let client = DoClient::new(config.token.clone());
    let create_req = CreateOneDropletReq {
        name: uuid::Uuid::new_v4().to_string(),
        region: Some("nyc1".into()),
        size: "s-1vcpu-1gb".into(),
        image: "ubuntu-22-04-x64".into(),
        ssh_keys: config.ssh_key_ids.clone(),
    };
    let create_resp = client.droplets().create(create_req).await?;
    let id = create_resp.droplet.id;
    let droplet = client.droplets().wait_creating(id).await?;

    Ok(droplet)
}

async fn delete_drop(config: &DoDockerConfig, id: usize) -> anyhow::Result<()> {
    let client = DoClient::new(config.token.clone());
    client.droplets().delete(&id.to_string()).await?;
    Ok(())
}

async fn do_session(ip: String, config: &DoDockerConfig) -> anyhow::Result<()> {
    let tcp = TcpStream::connect(format!("{ip}:22"))?;
    let mut sess = Session::new()?;
    sess.set_tcp_stream(tcp);
    sess.handshake()?;
    sess.userauth_pubkey_file("root", None, &Path::new(&config.ssh_prikey), None)?;

    sess.authenticated().then(|| println!("authed"));

    // upload file config.shell_file to remote
    let content = read_file(&config.shell_file)?;
    let mut send_c = sess.scp_send(
        &Path::new("/tmp/dodocker_shell.sh"),
        0o644,
        content.len() as u64,
        None,
    )?;
    send_c.write_all(content.as_bytes())?;
    // Close the channel and wait for the whole content to be transferred
    send_c.send_eof()?;
    send_c.wait_eof()?;
    send_c.close()?;
    send_c.wait_close()?;

    // run shell
    let mut channel = sess.channel_session()?;
    channel.exec("sh /tmp/dodocker_shell.sh")?;
    let mut s = String::new();
    channel.read_to_string(&mut s)?;
    println!("{}", s);
    channel.wait_close()?;
    println!("{}", channel.exit_status()?);

    Ok(())
}
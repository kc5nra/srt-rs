use failure::Error;
use srt::{ConnInitMethod, SrtSocketBuilder};
use std::time::Duration;
use tokio::time::delay_for;

async fn test_latency_exchange(
    connecter_latency: Duration,
    listener_latency: Duration,
) -> Result<(), Error> {
    let connecter = SrtSocketBuilder::new(ConnInitMethod::Connect("127.0.0.1:4000".parse()?))
        .latency(connecter_latency)
        .connect();

    let listener = SrtSocketBuilder::new(ConnInitMethod::Listen)
        .local_port(4000)
        .latency(listener_latency)
        .connect();

    let (connector, listener) = futures::try_join!(connecter, listener)?;

    let expected = Duration::max(connecter_latency, listener_latency);

    assert_eq!(connector.settings().tsbpd_latency, expected);
    assert_eq!(listener.settings().tsbpd_latency, expected);

    Ok(())
}

#[tokio::test]
async fn latency_exchange() -> Result<(), Error> {
    env_logger::init();

    test_latency_exchange(Duration::from_secs(3), Duration::from_secs(4)).await?;
    delay_for(Duration::from_secs(2)).await;
    test_latency_exchange(Duration::from_secs(4), Duration::from_secs(3)).await?;

    Ok(())
}

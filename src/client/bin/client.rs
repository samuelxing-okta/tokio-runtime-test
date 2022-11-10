use core::task;

use azuqua_core_queue_service::{QueueService, RabbitMQClient, RabbitMQClientConfig};

use lapin::uri::{AMQPAuthority, AMQPQueryString, AMQPScheme, AMQPUri, AMQPUserInfo};
use tokio::time::Duration;

const RUNTIME_QUEUE: &str = "runtime";

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    let total_rounds = &args[1];
    let total_rounds = total_rounds.parse::<u16>().unwrap();
    let invokes_per_round = &args[2];
    let invokes_per_round = invokes_per_round.parse::<u16>().unwrap();
    let secs_per_round = &args[3];
    let secs_per_round = secs_per_round.parse::<u64>().unwrap();

    // create a RabbitMQ client connected to localhost:<port>
    let heartbeat_period = Duration::from_millis(500);

    let uri = {
        let userinfo = AMQPUserInfo {
            username: "guest".to_string(),
            password: "guest".to_string(),
        };

        let authority = AMQPAuthority {
            userinfo,
            host: "localhost".to_string(),
            port: 5672,
        };

        let query = AMQPQueryString {
            frame_max: Some(131072),
            heartbeat: Some(60),
            ..Default::default()
        };

        AMQPUri {
            scheme: AMQPScheme::AMQP,
            authority,
            vhost: "/".to_string(),
            query,
        }
    };
    let config = RabbitMQClientConfig::new(heartbeat_period, uri);
    let amqp_client = RabbitMQClient::new(config).await;

    let mut tasks = Vec::new();
    for _ in 0..total_rounds {
        for _ in 0..invokes_per_round {
            // concurrently publish messages
            let c_amqp_client = amqp_client.clone();
            let jh = tokio::spawn(async move {
                let _ = c_amqp_client
                    .publish_payload_to_queue(RUNTIME_QUEUE.to_string(), "payload".into())
                    .await
                    .expect("Should have published");
            });
            tasks.push(jh);
        }
        tokio::time::sleep(Duration::from_secs(secs_per_round)).await;
    }

    for jh in tasks {
        jh.await.expect("The task being joined has panicked");
    }
}

use azuqua_core_queue_service::{Message, QueueService, RabbitMQClient, RabbitMQClientConfig};
use std::time::{Duration, Instant};

use lapin::uri::{AMQPAuthority, AMQPQueryString, AMQPScheme, AMQPUri, AMQPUserInfo};
use lazy_static::lazy_static;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use tokio::runtime::{Handle as Tokio1Handle, Runtime};
use tokio_stream::StreamExt as TokioStreamExt;

const RUNTIME_QUEUE: &str = "runtime";
const PREFETCH: u16 = 50;

lazy_static! {
    static ref MAIN_RUNTIME: StdRwLock<Option<Runtime>> = StdRwLock::new(Some(
        Runtime::new().expect("Error creating tokio main runtime.")
    ));
    static ref MAIN_HANDLE: Arc<Tokio1Handle> = Arc::new(
        MAIN_RUNTIME
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .handle()
            .clone()
    );
    static ref TELEMETRY_RUNTIME: StdRwLock<Option<Runtime>> = StdRwLock::new(Some(
        Runtime::new().expect("Error creating tokio telemetry runtime.")
    ));
    static ref TELEMETRY_HANDLE: Arc<Tokio1Handle> = Arc::new(
        TELEMETRY_RUNTIME
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .handle()
            .clone()
    );
}

fn main() {
    let init_ft = async move {
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

        RabbitMQClient::new(config).await
    };
    let tokio_1_handle = (**MAIN_HANDLE).clone();
    let amqp_client = tokio_1_handle.block_on(init_ft);
    println!("Initalized amqp client.");

    // declare the runtime queue
    let c_amqp_client = amqp_client.clone();
    let declare_ft = async move {
        let _ = c_amqp_client
            .queue_declare_durable(RUNTIME_QUEUE.to_string())
            .await;
        println!("Declared runtime queue.");
    };
    tokio_1_handle.block_on(declare_ft);

    // create the subscriber stream for consuming the messages
    let c_tokio_1_handle = (**MAIN_HANDLE).clone();
    let tokio_1_exclusive_handle = (**TELEMETRY_HANDLE).clone();
    let c_amqp_client = amqp_client.clone();
    let sub_ft = async move {
        let mut subscriber = c_amqp_client
            .subscribe_to_queue_manual_ack(RUNTIME_QUEUE.to_string(), PREFETCH)
            .await
            .expect("Should have subscribed with manual ack");
        while let Some(item) = subscriber.next().await {
            let message = {
                match item {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("Error when retrieving message: {:?}", e);
                        return;
                    }
                }
            };

            let ack_id = message.ack_id();
            let cc_amqp_client = c_amqp_client.clone();
            let _ = c_tokio_1_handle.spawn(async move {
                let _ = cc_amqp_client.ack(ack_id).await;
            });

            // spawn telemetry task and measuring the time till it complete
            let c_tokio_1_exclusive_handle = tokio_1_exclusive_handle.clone();
            tokio_1_exclusive_handle.spawn(async move {
                let start_time = Instant::now();
                let mock_telemetry_task = async move {
                    let duration = start_time.elapsed();
                    println!("{:?}, {:?}", duration.as_nanos(), duration);
                };
                let _ = c_tokio_1_exclusive_handle.spawn(mock_telemetry_task);
            });
            tokio::task::yield_now().await;
        }
    };
    let _ = tokio_1_handle.block_on(sub_ft);
}

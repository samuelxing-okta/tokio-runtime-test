use tokio::task;

async fn empty_await() -> i32 {
    // task::yield_now().await;
    0
}

#[tokio::main(flavor = "multi_thread", worker_threads = 6)]
async fn main() {
    for i in 0..100 {
        async {
            task::spawn(async move {
                // ...
                println!("spawned task {} done!", i)
            });

            // Yield, allowing the newly-spawned task to execute first.
            task::yield_now().await;
            println!("main task {} done!", i);
        }
        .await
    }

    tokio::time::timeout(std::time::Duration::from_millis(1), async move {
        let mut x = 0;
        loop {
            tokio::spawn(empty_await());
            x += 1;
            println!("x = {}", x);
            task::yield_now().await;
        }
    })
    .await
    .unwrap();

    println!("line never reached");
}

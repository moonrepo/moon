use moon_async_utils::run_pooled_blocking_tasks;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

mod run_pooled_blocking_tasks {
    use super::*;

    #[tokio::test]
    async fn applies_outputs_in_input_order() {
        let mut outputs = vec![];

        run_pooled_blocking_tasks(
            (0..100u64).collect(),
            |input| {
                // Earlier inputs finish later
                thread::sleep(Duration::from_micros(100 - input));

                Ok(input * 2)
            },
            |output| {
                outputs.push(output);
                Ok(())
            },
        )
        .await
        .unwrap();

        assert_eq!(outputs, (0..100u64).map(|i| i * 2).collect::<Vec<_>>());
    }

    #[tokio::test]
    async fn handles_empty_inputs() {
        let mut called = false;

        run_pooled_blocking_tasks(
            Vec::<u8>::new(),
            |_| Ok(()),
            |_| {
                called = true;
                Ok(())
            },
        )
        .await
        .unwrap();

        assert!(!called);
    }

    #[tokio::test]
    async fn returns_input_error_and_stops_pulling() {
        let processed = Arc::new(AtomicUsize::new(0));

        let error = run_pooled_blocking_tasks(
            (0..1000u32).collect(),
            {
                let processed = Arc::clone(&processed);

                move |input| {
                    processed.fetch_add(1, Ordering::SeqCst);

                    if *input == 0 {
                        return Err(miette::miette!("failed on {input}"));
                    }

                    thread::sleep(Duration::from_millis(1));

                    Ok(())
                }
            },
            |_| Ok(()),
        )
        .await
        .unwrap_err();

        assert_eq!(error.to_string(), "failed on 0");

        // Each worker stops after at most the input it was processing
        assert!(processed.load(Ordering::SeqCst) < 1000);
    }

    #[tokio::test]
    async fn returns_output_error() {
        let error = run_pooled_blocking_tasks(
            vec![1, 2, 3],
            |input| Ok(*input),
            |output| {
                if output == 2 {
                    return Err(miette::miette!("bad output {output}"));
                }

                Ok(())
            },
        )
        .await
        .unwrap_err();

        assert_eq!(error.to_string(), "bad output 2");
    }
}

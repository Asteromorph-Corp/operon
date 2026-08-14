// # Operon Example 4: Tracing & Output Capture Demo
//
// This example demonstrates operon's tracing integration:
//
// 1. `tracing::*` macros — structured logging with span context
// 2. `log` crate bridge — third-party crates using `log` are captured via tracing-log
// 3. `println!` / `eprintln!` capture — raw stdout/stderr is intercepted and forwarded
// 4. Custom `tracing::info_span!` — user-defined span context
//
// The pipeline simulates a simple sensor data processing flow:
//
//   Sensor  --aggregate-->  Summary  --report-->  Report
//
// Each task demonstrates a different logging facade to show how they all appear in the UI.
//
// To run:
//   POSTGRES_URI=<database_uri> cargo run --release --example ex4

use std::sync::Arc;

use async_trait::async_trait;
use operon::options::{OperonOptions, PsqlMetaStorageOptions, PsqlStorageOptions};
use operon::{Operon, OperonService, PsqlMetaStorage, define_operon};
use serde::{Deserialize, Serialize};

// ———————————————— Entity Definitions ———————————————— //

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Sensor {
    name: String,
    readings: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Summary {
    sensor_name: String,
    mean: f64,
    min: f64,
    max: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Report(String);

// ———————————————— Pipeline Definition ———————————————— //

define_operon! {
    sensors = {
        Sensor<sensor_id> = collect();
        Summary<batch_id> = aggregate(Sensor) for sensor_id;
        Report = report(Summary) for sensor_id, batch_id;
    }
}

// ———————————————— Service Implementation ———————————————— //

#[derive(OperonService)]
struct SensorService;

#[async_trait]
impl SensorsService for SensorService {
    /// Produces sensor data.
    /// Uses `tracing` — the native, recommended logging facade.
    async fn collect(&self) -> Result<Vec<Sensor>, Self::Error> {
        tracing::info!("Collecting sensor data");

        Ok(vec![
            Sensor {
                name: "temperature".into(),
                readings: vec![20.1, 21.3, 19.8, 22.0, 20.5],
            },
            Sensor {
                name: "humidity".into(),
                readings: vec![45.0, 47.2, 44.8, 46.1, 45.5],
            },
            Sensor {
                name: "pressure".into(),
                readings: vec![1013.0, 1012.5, 1013.2, 1012.8, 1013.1],
            },
        ])
    }

    /// Aggregates readings into summaries.
    /// Demonstrates all three non-native logging facades + custom spans:
    ///   - `log` crate (bridged via tracing-log)
    ///   - `println!` (captured stdout → WARN)
    ///   - `eprintln!` (captured stderr → ERROR)
    ///   - custom `tracing::info_span!` for user-defined context
    async fn aggregate(&self, sensor: Sensor) -> Result<Vec<Summary>, Self::Error> {
        // Custom span — wraps this job's work with structured context.
        // Any tracing events inside will carry [sensor=temperature] in the UI.
        let span = tracing::info_span!("aggregate", sensor = %sensor.name);
        let _guard = span.enter();

        // log crate — bridged to tracing via tracing-log::LogTracer
        log::info!("(log crate) aggregating sensor: {}", sensor.name);

        // println/eprintln — captured via fd redirect
        println!("(println) processing {} readings", sensor.readings.len());
        eprintln!("(eprintln) sensor range check for {}", sensor.name);

        let sum: f64 = sensor.readings.iter().sum();
        let mean = sum / sensor.readings.len() as f64;
        let min = sensor
            .readings
            .iter()
            .cloned()
            .fold(f64::INFINITY, f64::min);
        let max = sensor
            .readings
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);

        let summary = Summary {
            sensor_name: sensor.name.clone(),
            mean,
            min,
            max,
        };

        Ok(vec![summary.clone(), summary])
    }

    /// Generates reports.
    /// Uses `tracing` with structured fields to demonstrate span context.
    async fn report(&self, summary: Summary) -> Result<Report, Self::Error> {
        tracing::info!(
            sensor = %summary.sensor_name,
            mean = %format!("{:.1}", summary.mean),
            "Generating report"
        );

        tracing::debug!("Range: {:.1} - {:.1}", summary.min, summary.max);

        let text = format!(
            "{}: mean={:.1}, range=[{:.1}, {:.1}]",
            summary.sensor_name, summary.mean, summary.min, summary.max
        );

        Ok(Report(text))
    }
}

// ———————————————— Main ———————————————— //

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_uri = std::env::var("POSTGRES_URI")?;

    let operon_options = OperonOptions::new().with_log_dump("./dump");

    let service = Arc::new(SensorService);
    let storage = Arc::new(
        PsqlStorageOptions::new(&database_uri)
            .with_schema("ex4_data")
            .build::<PsqlSensorsStorage>()?,
    );
    let meta = PsqlMetaStorageOptions::new(&database_uri)
        .with_schema("ex4_meta")
        .build()?;

    let operon_instance: Operon<SensorService, PsqlSensorsStorage, PsqlMetaStorage> =
        Operon::new(service, storage.clone(), meta).with_options(operon_options);

    operon_instance.run().await?;

    // After the UI exits, print some results
    println!("Done! Checking stored reports...");
    if let Some(report) = storage.get_report([0, 0]).await? {
        println!("Report [0,0]: {}", report.0);
    }

    Ok(())
}

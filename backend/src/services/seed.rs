//! One-time seed of the initial course structure ("Light-Based Tactile Robotic Hand").
//!
//! Only the structure is seeded: modules, 48 week shells (all drafts) and the three
//! Week 1 day outlines. The instructor authors and publishes the remaining lessons.

use sqlx::SqlitePool;

use crate::{
    services::settings,
    util::{new_id, now, to_json},
};

const SEED_FLAG: &str = "seed:initial_course";

const MODULES: &[(&str, &str, [&str; 6])] = &[
    (
        "Foundations of Robotics and Electronics",
        "Core robotics concepts, DC circuits, components, instruments and prototyping skills.",
        [
            "Introduction to Robotics and Engineering",
            "DC Circuit Fundamentals",
            "Passive Components and Circuit Analysis",
            "Semiconductors: Diodes and Transistors",
            "Laboratory Instruments and Measurement Techniques",
            "Soldering, Prototyping, and Module Review",
        ],
    ),
    (
        "Python Programming for Engineers",
        "Python for measurement, data acquisition, analysis and visualization.",
        [
            "Python Fundamentals",
            "Control Flow, Functions, and Modules",
            "Numerical Computing with NumPy",
            "Plotting and Visualization",
            "Serial Communication and Data Acquisition",
            "Engineering Data Analysis Project",
        ],
    ),
    (
        "Embedded Systems and Microcontrollers",
        "Microcontroller architecture, peripherals, communication buses and firmware.",
        [
            "Microcontroller Architecture",
            "Digital I/O, Timers, and Interrupts",
            "Analog-to-Digital Conversion",
            "Communication Protocols: UART, I2C, and SPI",
            "PWM and Actuator Control",
            "Embedded Firmware Mini-Project",
        ],
    ),
    (
        "Photonics and Optical Sensing",
        "Light sources, photodetectors, light guiding and the principles of optical tactile sensing.",
        [
            "Nature of Light and Optical Principles",
            "Light Sources: LEDs and Laser Diodes",
            "Photodetectors: Photodiodes and Phototransistors",
            "Light Guiding in Waveguides and Elastomers",
            "Principles of Optical Tactile Sensing",
            "Building an Optical Sensing Prototype",
        ],
    ),
    (
        "Analog Signal Conditioning",
        "Amplification, filtering, noise management and calibration of sensor signals.",
        [
            "Operational Amplifier Fundamentals",
            "Transimpedance Amplifiers for Photodetectors",
            "Active and Passive Analog Filters",
            "Noise, Shielding, and Grounding",
            "Sensor Calibration and Characterization",
            "Signal Conditioning Board Design",
        ],
    ),
    (
        "Digital Signal Processing",
        "Sampling, digital filtering, spectral analysis and slip-detection algorithms.",
        [
            "Sampling Theory and Aliasing",
            "Digital Filtering: FIR and IIR",
            "Frequency-Domain Analysis and the FFT",
            "Feature Extraction from Tactile Signals",
            "Slip Detection Algorithms",
            "Real-Time DSP on Embedded Hardware",
        ],
    ),
    (
        "Robotics and Gripper Mechanics",
        "Kinematics, grasp mechanics, actuators and integrating tactile sensors into a gripper.",
        [
            "Robot Kinematics Fundamentals",
            "Grasping Mechanics and Friction",
            "Actuators and Motor Drivers",
            "Gripper Design and Fabrication",
            "Integrating Tactile Sensors into the Gripper",
            "Gripper Characterization and Testing",
        ],
    ),
    (
        "Feedback Control and System Integration",
        "Closed-loop control of grip force, slip-triggered adjustment and final integration.",
        [
            "Introduction to Feedback Control",
            "PID Control Design and Tuning",
            "Grip Force Control Loop",
            "Slip-Triggered Grip Adjustment",
            "System Integration and Validation",
            "Final Project Presentation and Review",
        ],
    ),
];

const WEEK1_LESSONS: &[(&str, &str, &[&str])] = &[
    (
        "Robotics, Sensors, and the Project Ahead",
        "What robotics is, the sense–think–act loop, and an overview of the tactile robotic hand project.",
        &["What is robotics?", "Introduction to sensors, controllers, and actuators.", "Project overview."],
    ),
    (
        "Human Touch and Tactile Robotic Sensing",
        "How human hands sense touch, how robots can emulate it, and basic laboratory safety.",
        &["How human hands sense touch.", "Overview of tactile robotic sensing.", "Basic laboratory safety."],
    ),
    (
        "Electronic Components and First Measurements",
        "Common electronic components, a multimeter demonstration, and the first practical exercise.",
        &["Introduction to electronic components.", "Multimeter demonstration.", "First practical engineering exercise."],
    ),
];

pub async fn seed_initial_course(pool: &SqlitePool) -> anyhow::Result<()> {
    if settings::get_flag(pool, SEED_FLAG).await? {
        return Ok(());
    }
    let slug = "light-based-tactile-robotic-hand";
    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM courses WHERE slug = ?").bind(slug).fetch_one(pool).await?;
    if exists > 0 {
        settings::set_flag(pool, SEED_FLAG).await?;
        return Ok(());
    }

    let ts = now();
    let instructor: Option<String> =
        sqlx::query_scalar("SELECT id FROM users WHERE role = 'admin' ORDER BY created_at LIMIT 1").fetch_optional(pool).await?;
    let course_id = new_id();
    let objectives = vec![
        "Design, build and characterize an optical tactile sensor.",
        "Condition and process photodetector signals in analog and digital domains.",
        "Develop embedded firmware for real-time data acquisition.",
        "Detect incipient object slippage from tactile signals.",
        "Implement closed-loop grip force control on a robotic gripper.",
        "Document experiments with professional laboratory reports.",
    ];

    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO courses (id, title, slug, description, difficulty, duration_weeks, days_per_week, instructor_id, status, objectives, created_at, updated_at)
         VALUES (?, ?, ?, ?, 'intermediate', 48, 3, ?, 'draft', ?, ?, ?)",
    )
    .bind(&course_id)
    .bind("Light-Based Tactile Robotic Hand")
    .bind(slug)
    .bind(
        "An interdisciplinary, hands-on engineering program focused on developing an optical tactile sensing system \
         that enables a robotic gripper to detect early signs of object slippage and automatically adjust its grip force.\n\n\
         The curriculum covers electronics, Python programming, embedded systems, photonics, optical sensing, analog signal \
         conditioning, digital signal processing, robotics, and feedback control.",
    )
    .bind(&instructor)
    .bind(to_json(&objectives))
    .bind(&ts)
    .bind(&ts)
    .execute(&mut *tx)
    .await?;

    let mut week_number = 0i64;
    for (pos, (title, desc, weeks)) in MODULES.iter().enumerate() {
        let module_id = new_id();
        sqlx::query(
            "INSERT INTO modules (id, course_id, title, description, position, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&module_id)
        .bind(&course_id)
        .bind(*title)
        .bind(*desc)
        .bind(pos as i64)
        .bind(&ts)
        .bind(&ts)
        .execute(&mut *tx)
        .await?;
        for week_title in weeks {
            week_number += 1;
            let week_id = new_id();
            sqlx::query(
                "INSERT INTO weeks (id, course_id, module_id, week_number, title, estimated_minutes, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&week_id)
            .bind(&course_id)
            .bind(&module_id)
            .bind(week_number)
            .bind(*week_title)
            .bind(if week_number == 1 { 540 } else { 0 })
            .bind(&ts)
            .bind(&ts)
            .execute(&mut *tx)
            .await?;

            if week_number == 1 {
                for (i, (lt, summary, points)) in WEEK1_LESSONS.iter().enumerate() {
                    let outline = points.iter().map(|p| format!("- {p}")).collect::<Vec<_>>().join("\n");
                    let content = format!("## Session outline\n\n{outline}\n");
                    sqlx::query(
                        "INSERT INTO lessons (id, week_id, day_number, position, title, summary, content_md, objectives, estimated_minutes, created_at, updated_at)
                         VALUES (?, ?, ?, ?, ?, ?, ?, ?, 180, ?, ?)",
                    )
                    .bind(new_id())
                    .bind(&week_id)
                    .bind(i as i64 + 1)
                    .bind(i as i64)
                    .bind(*lt)
                    .bind(*summary)
                    .bind(content)
                    .bind(to_json(points))
                    .bind(&ts)
                    .bind(&ts)
                    .execute(&mut *tx)
                    .await?;
                }
            }
        }
    }
    tx.commit().await?;
    settings::set_flag(pool, SEED_FLAG).await?;
    tracing::info!("seeded initial course structure (48 draft weeks)");
    Ok(())
}

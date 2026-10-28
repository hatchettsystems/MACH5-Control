//! Block-based printer config. Each Klipper config section becomes a typed module with
//! documented, range-checked parameters; the machine config is a list of elected modules
//! grouped by system, instead of one long text file. Parameter names follow the Klipper
//! config reference so existing knowledge carries over; Klipper's `*_pin` options become
//! bindings to a CAN node.
#[derive(Clone, PartialEq, Debug)]
pub enum PVal {
    F(f64),
    I(i64),
    B(bool),
    S(String),
    C(usize),
}

#[derive(Clone)]
pub enum PKind {
    F { unit: &'static str, min: f64, max: f64 },
    I { unit: &'static str, min: i64, max: i64 },
    B,
    S,
    C(&'static [&'static str]),
}

pub struct ParamDef {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: PKind,
    pub def: PVal,
    pub desc: &'static str,
}

pub struct ModDef {
    pub id: &'static str,
    pub title: &'static str,
    /// Klipper section name; `{}` is replaced by the instance name.
    pub fmt: &'static str,
    pub group: usize,
    pub multi: bool,
    pub desc: &'static str,
    pub requires: &'static [&'static str],
    pub params: Vec<ParamDef>,
}

#[derive(Clone)]
pub struct Block {
    pub uid: u32,
    pub def: usize,
    pub name: String,
    pub on: bool,
    pub vals: Vec<PVal>,
}

pub const GROUPS: [(&str, &str); 8] = [
    ("Motion & kinematics", "Machine limits, drive axes, extruder motion, input shaping, homing"),
    ("Heating & thermal", "Hotend, bed and chamber heaters, temperature sensors"),
    ("Cooling & fans", "Part cooling, heater-linked fans, controller and temperature fans"),
    ("Probing & leveling", "Eddy probe, bed mesh, gantry leveling, axis twist"),
    ("Sensors & filament", "Filament sensors, accelerometers, resonance test, MMU"),
    ("Lighting & I/O", "LEDs, output pins, on-machine display"),
    ("Print flow, macros & safety", "Retraction, idle and pause behaviour, heater checks, power-loss recovery"),
    ("Communication & nodes", "CAN FD segments and the nodes bound to them"),
];

const NODE_DRIVE: &[&str] = &["Drive X", "Drive Y", "Drive Z1", "Drive Z2", "Drive Z3", "Drive E"];
const NODE_HEAT: &[&str] = &["Toolhead", "Mainboard", "Chamber IO"];
const NODE_FAN: &[&str] = &["Toolhead", "Chamber IO", "Mainboard"];
const NODE_ALL: &[&str] = &["Mainboard", "Toolhead", "Chamber IO", "Eddy Probe", "Power Board", "MMU", "Drive X", "Drive Y", "Drive Z1", "Drive Z2", "Drive Z3", "Drive E"];
const SENSORS: &[&str] = &["PT1000", "PT100 INA826", "ATC Semitec 104GT-2", "NTC 100K MGB18", "Generic 3950", "temperature_mcu"];
const CONTROL: &[&str] = &["pid", "mpc", "watermark"];
const SHAPERS: &[&str] = &["zv", "mzv", "zvd", "ei", "2hump_ei", "3hump_ei"];
const KIN: &[&str] = &["corexy", "cartesian", "corexz", "delta"];

fn pf(key: &'static str, label: &'static str, def: f64, unit: &'static str, min: f64, max: f64, desc: &'static str) -> ParamDef {
    ParamDef { key, label, kind: PKind::F { unit, min, max }, def: PVal::F(def), desc }
}
fn pi(key: &'static str, label: &'static str, def: i64, unit: &'static str, min: i64, max: i64, desc: &'static str) -> ParamDef {
    ParamDef { key, label, kind: PKind::I { unit, min, max }, def: PVal::I(def), desc }
}
fn pb(key: &'static str, label: &'static str, def: bool, desc: &'static str) -> ParamDef {
    ParamDef { key, label, kind: PKind::B, def: PVal::B(def), desc }
}
fn ps(key: &'static str, label: &'static str, def: &str, desc: &'static str) -> ParamDef {
    ParamDef { key, label, kind: PKind::S, def: PVal::S(def.into()), desc }
}
fn pc(key: &'static str, label: &'static str, opts: &'static [&'static str], def: usize, desc: &'static str) -> ParamDef {
    ParamDef { key, label, kind: PKind::C(opts), def: PVal::C(def), desc }
}

fn heater_params(max_default: f64, node: &'static [&'static str]) -> Vec<ParamDef> {
    vec![
        pc("node", "Heater node", node, 0, "CAN node that drives the heater output. Replaces Klipper's heater_pin."),
        pc("sensor_type", "Sensor type", SENSORS, 0, "Thermistor or RTD type read by the node. Replaces sensor_pin."),
        pc("control", "Control", CONTROL, 0, "Control algorithm. pid, mpc (model predictive) or watermark (bang-bang with hysteresis)."),
        pf("pid_Kp", "PID Kp", 22.2, "", 0.0, 1000.0, "Proportional gain (used when control is pid)."),
        pf("pid_Ki", "PID Ki", 1.08, "", 0.0, 1000.0, "Integral gain."),
        pf("pid_Kd", "PID Kd", 114.0, "", 0.0, 5000.0, "Derivative gain."),
        pf("max_power", "Max power", 1.0, "", 0.0, 1.0, "Maximum heater duty (0..1)."),
        pf("min_temp", "Min temp", 0.0, "C", -50.0, 100.0, "Below this reading the heater is shut off (sensor fault)."),
        pf("max_temp", "Max temp", max_default, "C", 30.0, 500.0, "Hard cutoff enforced on the node and the STM32, independent of the main controller."),
        pi("watchdog_timeout_ms", "Heartbeat timeout", 1500, "ms", 100, 10000, "The node cuts this heater if the Host heartbeat is missing this long. (MACH5)"),
    ]
}

/// The module catalog.
pub fn defs() -> Vec<ModDef> {
    let m = |id, title, fmt, group, multi, desc, requires, params| ModDef { id, title, fmt, group, multi, desc, requires, params };
    vec![
        // ---- 0 Motion & kinematics ----
        m("printer", "Printer limits", "printer", 0, false, "Kinematics and global motion limits.", &[], vec![
            pc("kinematics", "Kinematics", KIN, 0, "Motion model."),
            pf("max_velocity", "Max velocity", 500.0, "mm/s", 10.0, 1500.0, "Toolhead velocity limit."),
            pf("max_accel", "Max acceleration", 6000.0, "mm/s2", 100.0, 40000.0, "Toolhead acceleration limit."),
            pf("max_jerk", "Max jerk", 90000.0, "mm/s3", 1000.0, 1.0e6, "Third-order planner jerk limit. (MACH5)"),
            pf("minimum_cruise_ratio", "Min cruise ratio", 0.5, "", 0.0, 1.0, "Fraction of a move kept at cruise speed before decelerating."),
            pf("square_corner_velocity", "Corner velocity", 5.0, "mm/s", 0.0, 50.0, "Cornering speed. The MACH5 planner blends corners instead of using junction deviation."),
            pf("max_z_velocity", "Max Z velocity", 40.0, "mm/s", 1.0, 200.0, "Z axis velocity limit."),
            pf("max_z_accel", "Max Z acceleration", 400.0, "mm/s2", 10.0, 5000.0, "Z axis acceleration limit."),
        ]),
        m("stepper", "Drive axis", "stepper_{}", 0, true, "One FOC step-servo driver axis (Klipper [stepper_x] etc.). Name it x, y, z, z1 ...", &[], vec![
            pc("node", "Drive node", NODE_DRIVE, 0, "CAN FD A node for this axis. Replaces step_pin, dir_pin and enable_pin."),
            pf("rotation_distance", "Rotation distance", 40.0, "mm", 0.1, 1000.0, "Distance the axis moves per full motor rotation."),
            pi("full_steps_per_rotation", "Full steps / rotation", 200, "", 100, 800, "200 for 1.8 degree motors, 400 for 0.9 degree."),
            ps("gear_ratio", "Gear ratio", "", "Gearbox ratio such as 5:1 or 57:11, empty for direct drive."),
            pf("position_endstop", "Endstop position", 0.0, "mm", -500.0, 1000.0, "Axis position at the endstop."),
            pf("position_min", "Min position", 0.0, "mm", -500.0, 1000.0, "Minimum valid position."),
            pf("position_max", "Max position", 350.0, "mm", 10.0, 1500.0, "Maximum valid position."),
            pf("homing_speed", "Homing speed", 50.0, "mm/s", 1.0, 300.0, "Speed of the first homing move."),
            pf("homing_retract_dist", "Homing retract", 5.0, "mm", 0.0, 50.0, "Distance to back off after the first homing move."),
            pf("second_homing_speed", "Second homing speed", 25.0, "mm/s", 1.0, 150.0, "Slower speed for the precise second homing move."),
            pc("endstop_mode", "Endstop mode", &["sensorless (FOC)", "switch via node", "probe"], 0, "How this axis finds its home position."),
        ]),
        m("extruder_motion", "Extruder motion", "extruder", 0, false, "Extruder drive and filament geometry. Shares the [extruder] section with the hotend heater.", &[], vec![
            pc("drive_node", "Drive node", NODE_DRIVE, 5, "Extruder drive node."),
            pf("nozzle_diameter", "Nozzle diameter", 0.4, "mm", 0.1, 2.0, "Nozzle orifice diameter."),
            pf("filament_diameter", "Filament diameter", 1.75, "mm", 1.0, 3.0, "Filament diameter."),
            pf("rotation_distance", "Rotation distance", 4.637, "mm", 0.1, 100.0, "Filament length per motor rotation."),
            pf("max_extrude_only_distance", "Max extrude-only", 50.0, "mm", 1.0, 500.0, "Longest single extrude or retract without XYZ motion."),
            pf("max_extrude_cross_section", "Max cross-section", 0.64, "mm2", 0.1, 5.0, "Largest allowed extrusion cross-section."),
            pf("instantaneous_corner_velocity", "Corner velocity", 1.0, "mm/s", 0.0, 20.0, "Extruder velocity change allowed at a corner."),
            pf("pressure_advance", "Pressure advance", 0.044, "s", 0.0, 1.0, "Pressure advance applied inside the planner. Also set per filament profile."),
            pf("pressure_advance_smooth_time", "PA smooth time", 0.04, "s", 0.0, 0.2, "Averaging window for pressure advance."),
        ]),
        m("input_shaper", "Input shaper", "input_shaper", 0, false, "Resonance compensation applied inside the planner.", &[], vec![
            pc("shaper_type_x", "X shaper", SHAPERS, 1, "Shaper algorithm for X."),
            pf("shaper_freq_x", "X frequency", 48.2, "Hz", 0.0, 200.0, "Resonance frequency of X."),
            pf("damping_ratio_x", "X damping", 0.1, "", 0.0, 0.5, "Damping ratio of X."),
            pc("shaper_type_y", "Y shaper", SHAPERS, 3, "Shaper algorithm for Y."),
            pf("shaper_freq_y", "Y frequency", 41.6, "Hz", 0.0, 200.0, "Resonance frequency of Y."),
            pf("damping_ratio_y", "Y damping", 0.1, "", 0.0, 0.5, "Damping ratio of Y."),
            pb("auto_apply", "Auto-apply results", true, "Apply resonance test results automatically. (MACH5)"),
        ]),
        m("safe_z_home", "Safe Z home", "safe_z_home", 0, false, "Home Z at a safe XY position, with a lift before XY homing.", &[], vec![
            ps("home_xy_position", "Home XY position", "175, 175", "XY position to home Z at."),
            pf("speed", "Travel speed", 100.0, "mm/s", 1.0, 500.0, "Speed to reach the position."),
            pf("z_hop", "Z hop", 10.0, "mm", 0.0, 50.0, "Lift before homing."),
            pf("z_hop_speed", "Z hop speed", 15.0, "mm/s", 1.0, 100.0, "Speed of the lift."),
            pb("move_to_previous", "Return to previous", false, "Return to the starting XY after homing Z."),
        ]),
        // ---- 1 Heating & thermal ----
        m("hotend", "Hotend heater", "extruder", 1, false, "Hotend heater and sensor. Shares the [extruder] section with the extruder motion module.", &[], {
            let mut p = heater_params(300.0, NODE_HEAT);
            p[0].key = "heater_node";
            p.push(pf("min_extrude_temp", "Min extrude temp", 170.0, "C", 0.0, 300.0, "Extrusion is refused below this temperature."));
            p
        }),
        m("heater_bed", "Bed heater", "heater_bed", 1, false, "Heated bed.", &[], { let mut p = heater_params(120.0, NODE_HEAT); p[0].def = PVal::C(1); p[3].def = PVal::F(54.0); p[4].def = PVal::F(0.8); p[5].def = PVal::F(325.0); p }),
        m("heater_generic", "Generic heater (chamber)", "heater_generic {}", 1, true, "Any extra heater, such as a chamber heater.", &[], {
            let mut p = heater_params(80.0, NODE_HEAT);
            p[0].def = PVal::C(2);
            p.push(ps("gcode_id", "G-code id", "C", "Id used to report this sensor in G-code."));
            p
        }),
        m("temperature_sensor", "Temperature sensor", "temperature_sensor {}", 1, true, "A sensor with no heater: MCU temperature, enclosure, driver.", &[], vec![
            pc("sensor_type", "Sensor type", SENSORS, 5, "Sensor type."),
            pc("node", "Sensor node", NODE_ALL, 0, "Node that reads the sensor."),
            pf("min_temp", "Min temp", 0.0, "C", -50.0, 100.0, "Lowest valid reading."),
            pf("max_temp", "Max temp", 100.0, "C", 30.0, 300.0, "Highest valid reading."),
            ps("gcode_id", "G-code id", "", "Id used to report this sensor."),
        ]),
        // ---- 2 Cooling & fans ----
        m("fan", "Part cooling fan", "fan", 2, false, "Print cooling fan controlled by M106 / M107.", &[], vec![
            pc("node", "Fan node", NODE_FAN, 0, "Node that drives the fan. Replaces pin."),
            pf("max_power", "Max power", 1.0, "", 0.0, 1.0, "Maximum duty (0..1)."),
            pf("kick_start_time", "Kick start", 0.1, "s", 0.0, 5.0, "Full power burst when starting from stop."),
            pf("off_below", "Off below", 0.1, "", 0.0, 1.0, "Duty below which the fan turns off."),
            pf("cycle_time", "PWM cycle", 0.01, "s", 0.0005, 0.1, "PWM period."),
            pb("hardware_pwm", "Hardware PWM", false, "Use hardware PWM on the node."),
            pb("tachometer", "Tachometer", true, "Read the fan tach signal."),
        ]),
        m("heater_fan", "Heater fan", "heater_fan {}", 2, true, "Fan that runs when a heater is hot (heatsink fan).", &["hotend"], vec![
            pc("node", "Fan node", NODE_FAN, 0, "Node that drives the fan."),
            pc("heater", "Heater", &["extruder", "heater_bed", "chamber"], 0, "Heater that triggers this fan."),
            pf("heater_temp", "Trigger temp", 50.0, "C", 0.0, 300.0, "Fan runs above this heater temperature."),
            pf("fan_speed", "Fan speed", 1.0, "", 0.0, 1.0, "Duty when active."),
            pf("shutdown_speed", "Shutdown speed", 1.0, "", 0.0, 1.0, "Duty if the machine faults."),
        ]),
        m("controller_fan", "Controller fan", "controller_fan {}", 2, true, "Fan that runs while drives or heaters are active.", &[], vec![
            pc("node", "Fan node", NODE_FAN, 2, "Node that drives the fan."),
            pi("idle_timeout", "Idle timeout", 30, "s", 0, 3600, "Keep running this long after activity stops."),
            pf("idle_speed", "Idle speed", 0.3, "", 0.0, 1.0, "Duty while idle."),
            pf("fan_speed", "Fan speed", 1.0, "", 0.0, 1.0, "Duty while active."),
            ps("stepper", "Watched drives", "x, y, z", "Drives that keep the fan running."),
        ]),
        m("fan_generic", "Generic fan", "fan_generic {}", 2, true, "Any other fan controlled from G-code (exhaust, filter).", &[], vec![
            pc("node", "Fan node", NODE_FAN, 1, "Node that drives the fan."),
            pf("max_power", "Max power", 1.0, "", 0.0, 1.0, "Maximum duty."),
            pf("kick_start_time", "Kick start", 0.1, "s", 0.0, 5.0, "Start burst."),
            pf("off_below", "Off below", 0.1, "", 0.0, 1.0, "Duty below which the fan turns off."),
        ]),
        m("temperature_fan", "Temperature fan", "temperature_fan {}", 2, true, "Fan regulated to hold a temperature (chamber filter).", &[], vec![
            pc("node", "Fan node", NODE_FAN, 1, "Node that drives the fan."),
            pc("sensor_type", "Sensor type", SENSORS, 0, "Sensor type."),
            pf("target_temp", "Target temp", 40.0, "C", 0.0, 100.0, "Temperature to hold."),
            pf("max_speed", "Max speed", 1.0, "", 0.0, 1.0, "Upper duty limit."),
            pf("min_speed", "Min speed", 0.3, "", 0.0, 1.0, "Lower duty limit."),
            pc("control", "Control", CONTROL, 2, "Control algorithm."),
        ]),
        // ---- 3 Probing & leveling ----
        m("probe", "Eddy probe", "probe", 3, false, "Eddy-current probe on its own CAN node, with temperature compensation.", &[], vec![
            pc("driver", "Driver", &["mach5_eddy", "beacon", "cartographer"], 0, "Beacon and Cartographer are compatibility drivers only."),
            pc("node", "Probe node", NODE_ALL, 3, "Node hosting the probe."),
            pf("x_offset", "X offset", 0.0, "mm", -100.0, 100.0, "Probe X offset from the nozzle."),
            pf("y_offset", "Y offset", 22.0, "mm", -100.0, 100.0, "Probe Y offset from the nozzle."),
            pf("z_offset", "Z offset", -0.012, "mm", -3.0, 3.0, "Nozzle height at trigger."),
            pf("speed", "Probe speed", 5.0, "mm/s", 0.5, 50.0, "Descent speed while probing."),
            pf("lift_speed", "Lift speed", 10.0, "mm/s", 0.5, 100.0, "Speed between samples."),
            pi("samples", "Samples", 5, "", 1, 50, "Samples per point."),
            pf("sample_retract_dist", "Sample retract", 2.0, "mm", 0.0, 20.0, "Lift between samples."),
            pc("samples_result", "Result", &["median", "average"], 0, "How samples are combined."),
            pf("samples_tolerance", "Tolerance", 0.006, "mm", 0.0005, 0.1, "Maximum spread between samples."),
            pi("samples_tolerance_retries", "Tolerance retries", 3, "", 0, 20, "Retries when the spread is too large."),
            pb("thermal_compensation", "Thermal compensation", true, "Compensate Z for probe temperature. (MACH5)"),
        ]),
        m("bed_mesh", "Bed mesh", "bed_mesh", 3, false, "Height map of the bed applied during the print.", &["probe"], vec![
            pf("speed", "Travel speed", 150.0, "mm/s", 1.0, 500.0, "Speed between mesh points."),
            pf("horizontal_move_z", "Travel height", 2.0, "mm", 0.5, 20.0, "Height while travelling between points."),
            ps("mesh_min", "Mesh min", "20, 20", "Minimum probed XY (probe coordinates)."),
            ps("mesh_max", "Mesh max", "330, 330", "Maximum probed XY."),
            ps("probe_count", "Probe count", "9, 9", "Points along X and Y."),
            pc("algorithm", "Algorithm", &["bicubic", "lagrange"], 0, "Interpolation algorithm."),
            pf("fade_start", "Fade start", 1.0, "mm", 0.0, 100.0, "Height where mesh correction starts to fade."),
            pf("fade_end", "Fade end", 10.0, "mm", 0.0, 200.0, "Height where correction reaches zero (0 disables fade)."),
            pf("adaptive_margin", "Adaptive margin", 5.0, "mm", 0.0, 50.0, "Margin around the print footprint when adaptive mesh is on."),
            pb("adaptive", "Adaptive mesh", true, "Probe only the print footprint taken from G-code metadata. (MACH5)"),
        ]),
        m("z_tilt", "Z tilt (gantry leveling)", "z_tilt", 3, false, "Level the gantry with the probe. Works with any number of Z motors.", &["probe"], vec![
            ps("z_positions", "Z positions", "-50, 18\n175, 360\n400, 18", "Positions of the Z motors/pivots."),
            ps("points", "Probe points", "30, 30\n175, 320\n320, 30", "Points probed, one per Z motor."),
            pf("speed", "Travel speed", 200.0, "mm/s", 1.0, 500.0, "Speed between points."),
            pf("horizontal_move_z", "Travel height", 5.0, "mm", 0.5, 30.0, "Height while travelling."),
            pi("retries", "Retries", 5, "", 0, 20, "Maximum attempts."),
            pf("retry_tolerance", "Retry tolerance", 0.0075, "mm", 0.0, 0.1, "Stop retrying when within this tolerance."),
        ]),
        m("axis_twist", "Axis twist compensation", "axis_twist_compensation", 3, false, "Correct X-axis twist measured along the rail.", &["probe"], vec![
            pf("calibrate_start_x", "Start X", 20.0, "mm", 0.0, 1000.0, "First calibration X."),
            pf("calibrate_end_x", "End X", 330.0, "mm", 0.0, 1000.0, "Last calibration X."),
            pf("calibrate_y", "Y position", 175.0, "mm", 0.0, 1000.0, "Y used for the measurement."),
            pf("speed", "Travel speed", 50.0, "mm/s", 1.0, 300.0, "Speed between points."),
            pf("horizontal_move_z", "Travel height", 5.0, "mm", 0.5, 30.0, "Height while travelling."),
        ]),
        m("screws_tilt_adjust", "Bed screw adjustment", "screws_tilt_adjust", 3, false, "Guided bed screw adjustment using the probe.", &["probe"], vec![
            ps("screw1", "Screw 1", "30, 30", "XY of screw 1 (the reference)."),
            ps("screw1_name", "Screw 1 name", "front left", "Label shown in the UI."),
            ps("screw2", "Screw 2", "320, 30", "XY of screw 2."),
            ps("screw3", "Screw 3", "175, 320", "XY of screw 3."),
            pc("screw_thread", "Thread", &["CW-M3", "CCW-M3", "CW-M4", "CCW-M4", "CW-M5", "CCW-M5"], 0, "Thread direction and size."),
            pf("speed", "Travel speed", 150.0, "mm/s", 1.0, 500.0, "Speed between screws."),
        ]),
        // ---- 4 Sensors & filament ----
        m("filament_switch_sensor", "Filament switch sensor", "filament_switch_sensor {}", 4, true, "Runout / insert detection.", &[], vec![
            pc("node", "Sensor node", NODE_ALL, 1, "Node reading the switch. Replaces switch_pin."),
            pb("pause_on_runout", "Pause on runout", true, "Pause the print when filament runs out."),
            pf("event_delay", "Event delay", 3.0, "s", 0.0, 60.0, "Minimum time between events."),
            pf("pause_delay", "Pause delay", 0.5, "s", 0.0, 10.0, "Delay between pause and the runout rule."),
            ps("runout_gcode", "Runout rule", "", "Name of the rule to run on runout (sandboxed, not a shell macro)."),
            ps("insert_gcode", "Insert rule", "", "Rule to run when filament is inserted."),
        ]),
        m("filament_motion_sensor", "Filament motion sensor", "filament_motion_sensor {}", 4, true, "Detects clogs or tangles from filament movement.", &[], vec![
            pc("node", "Sensor node", NODE_ALL, 1, "Node reading the sensor."),
            pf("detection_length", "Detection length", 7.0, "mm", 1.0, 50.0, "Filament length without movement before triggering."),
            pb("pause_on_runout", "Pause on runout", true, "Pause the print on trigger."),
        ]),
        m("adxl345", "Accelerometer (ADXL345)", "adxl345 {}", 4, true, "Accelerometer for resonance testing.", &[], vec![
            pc("node", "Sensor node", &["Eddy Probe", "Wireless ADXL"], 0, "Probe-board ADXL on CAN FD B, or the wireless module over WiFi."),
            ps("axes_map", "Axes map", "x, y, z", "Mapping from chip axes to machine axes."),
            pc("rate", "Data rate", &["3200", "1600", "800", "400"], 0, "Samples per second."),
        ]),
        m("resonance_tester", "Resonance tester", "resonance_tester", 4, false, "Resonance test sweep.", &["adxl345"], vec![
            ps("probe_points", "Probe points", "175, 175, 20", "Where the toolhead sits during the test."),
            pf("min_freq", "Min frequency", 5.0, "Hz", 1.0, 100.0, "Start of the sweep."),
            pf("max_freq", "Max frequency", 135.0, "Hz", 20.0, 300.0, "End of the sweep."),
            pf("accel_per_hz", "Accel per Hz", 75.0, "mm/s2", 10.0, 500.0, "Acceleration rise per Hz."),
            pf("hz_per_sec", "Sweep rate", 1.0, "Hz/s", 0.1, 10.0, "Frequency change rate."),
        ]),
        m("mmu", "Multi-material unit", "mmu", 4, false, "Multi-material node with its own state machine and spool tracking.", &[], vec![
            pc("node", "MMU node", NODE_ALL, 5, "CAN FD B node."),
            pi("slot_count", "Slots", 8, "", 1, 16, "Filament slots."),
            pf("bowden_length", "Bowden length", 640.0, "mm", 0.0, 3000.0, "Length from MMU to extruder."),
            pf("load_speed", "Load speed", 80.0, "mm/s", 1.0, 300.0, "Feed speed."),
            pf("unload_speed", "Unload speed", 100.0, "mm/s", 1.0, 300.0, "Retract speed."),
            pb("spool_tracking", "Spool tracking", true, "Track remaining filament per spool."),
        ]),
        // ---- 5 Lighting & I/O ----
        m("neopixel", "LED strip", "neopixel {}", 5, true, "Addressable LEDs.", &[], vec![
            pc("node", "LED node", NODE_FAN, 0, "Node that drives the LED data line."),
            pi("chain_count", "LED count", 12, "", 1, 256, "Number of LEDs."),
            pc("color_order", "Colour order", &["GRB", "RGB", "GRBW"], 0, "Colour order of the LED chip."),
            pf("initial_RED", "Initial red", 0.6, "", 0.0, 1.0, "Initial red level."),
            pf("initial_GREEN", "Initial green", 0.6, "", 0.0, 1.0, "Initial green level."),
            pf("initial_BLUE", "Initial blue", 0.6, "", 0.0, 1.0, "Initial blue level."),
        ]),
        m("output_pin", "Output", "output_pin {}", 5, true, "A plain on/off or PWM output.", &[], vec![
            pc("node", "Output node", NODE_ALL, 0, "Node providing the output."),
            pb("pwm", "PWM", false, "Use PWM instead of on/off."),
            pf("value", "Initial value", 0.0, "", 0.0, 1.0, "Value at startup."),
            pf("shutdown_value", "Shutdown value", 0.0, "", 0.0, 1.0, "Value after a fault."),
            pf("cycle_time", "PWM cycle", 0.01, "s", 0.0005, 0.1, "PWM period."),
        ]),
        m("display", "On-machine display", "display", 5, false, "DSI touchscreen on the mainboard.", &[], vec![
            pf("brightness", "Brightness", 0.8, "", 0.05, 1.0, "Backlight level."),
            pi("sleep_after", "Sleep after", 300, "s", 0, 7200, "Dim after this long idle (0 = never)."),
            pc("theme", "Theme", &["dark", "light"], 0, "Screen theme."),
        ]),
        // ---- 6 Print flow, macros & safety ----
        m("virtual_sdcard", "SD card printing", "virtual_sdcard", 6, false, "Where print files live.", &[], vec![
            ps("path", "Path", "/sd/gcodes", "Directory of G-code files on the SD card."),
            ps("on_error_gcode", "On error rule", "", "Rule to run if a print errors."),
        ]),
        m("exclude_object", "Exclude object", "exclude_object", 6, false, "Cancel individual objects mid-print using planner-level boundaries.", &[], vec![
            pb("enabled", "Enabled", true, "Read object boundaries from G-code metadata."),
        ]),
        m("firmware_retraction", "Firmware retraction", "firmware_retraction", 6, false, "G10/G11 retraction handled by the planner.", &[], vec![
            pf("retract_length", "Retract length", 0.8, "mm", 0.0, 20.0, "Retraction length."),
            pf("retract_speed", "Retract speed", 35.0, "mm/s", 1.0, 200.0, "Retract speed."),
            pf("unretract_extra_length", "Unretract extra", 0.0, "mm", -5.0, 5.0, "Extra length when unretracting."),
            pf("unretract_speed", "Unretract speed", 35.0, "mm/s", 1.0, 200.0, "Unretract speed."),
        ]),
        m("idle_timeout", "Idle timeout", "idle_timeout", 6, false, "What happens when the machine sits idle.", &[], vec![
            pi("timeout", "Timeout", 600, "s", 1, 86400, "Idle time before the rule runs."),
            ps("gcode", "Idle rule", "heaters.off", "Rule to run (turn heaters and drives off)."),
        ]),
        m("pause_resume", "Pause / resume", "pause_resume", 6, false, "Pause and resume behaviour.", &[], vec![
            pf("recover_velocity", "Recover velocity", 50.0, "mm/s", 1.0, 500.0, "Speed when returning to the print after a pause."),
        ]),
        m("gcode_macro", "Rule (macro)", "gcode_macro {}", 6, true, "A sandboxed rule replaces a shell-style macro. Edit it in Rules.", &[], vec![
            ps("description", "Description", "", "Shown in the UI."),
            pc("trigger", "Trigger", &["manual", "job start", "sensor event", "material change", "idle timer"], 0, "What starts the rule."),
            ps("gcode", "Actions", "", "Actions run in order, inside the sandbox."),
        ]),
        m("delayed_gcode", "Delayed rule", "delayed_gcode {}", 6, true, "Run a rule after a delay.", &[], vec![
            pf("initial_duration", "Initial delay", 0.0, "s", 0.0, 86400.0, "Delay after startup (0 = disabled)."),
            ps("gcode", "Actions", "", "Actions to run."),
        ]),
        m("verify_heater", "Heater safety check", "verify_heater {}", 6, true, "Detect a heater that fails to heat. Name it after the heater.", &[], vec![
            pf("max_error", "Max error", 120.0, "", 0.0, 1000.0, "Accumulated error before a fault is raised."),
            pf("check_gain_time", "Gain check time", 20.0, "s", 1.0, 600.0, "Time to wait for expected temperature gain."),
            pf("hysteresis", "Hysteresis", 5.0, "C", 0.0, 50.0, "Allowed difference from target."),
            pf("heating_gain", "Heating gain", 2.0, "C", 0.0, 50.0, "Minimum gain expected while heating."),
        ]),
        m("power_loss", "Power-loss recovery", "power_loss", 6, false, "Checkpoint the print to flash and resume after a power loss. (MACH5)", &[], vec![
            pb("checkpoint", "Checkpoint to flash", true, "Write print state to flash."),
            pi("every_layers", "Checkpoint every", 1, "layers", 1, 100, "Layer interval."),
            pf("resume_z_hop", "Resume Z hop", 2.0, "mm", 0.0, 20.0, "Lift before resuming."),
            pb("restore_thermal", "Restore heaters", true, "Reheat to the saved temperatures before resuming."),
        ]),
        // ---- 7 Communication & nodes ----
        m("canbus", "CAN FD segment", "canbus {}", 7, true, "One physical CAN FD segment. Name it a (drive) or b (peripherals).", &[], vec![
            pc("segment", "Segment", &["A - drive", "B - peripherals"], 0, "Physical segment."),
            pi("arbitration_bitrate", "Arbitration bitrate", 1_000_000, "bit/s", 125_000, 1_000_000, "Arbitration phase bitrate."),
            pi("data_bitrate", "Data bitrate", 5_000_000, "bit/s", 1_000_000, 8_000_000, "Data phase bitrate."),
            pc("payload", "Payload", &["64 bytes", "32 bytes", "8 bytes"], 0, "Maximum frame payload."),
            pf("time_sync_rate", "Time sync rate", 50.0, "Hz", 10.0, 100.0, "Master clock broadcast rate."),
        ]),
        m("mcu", "Node (MCU)", "mcu {}", 7, true, "A CAN node bound to a short address. Replaces Klipper's serial / canbus_uuid [mcu].", &["canbus"], vec![
            pc("class", "Node class", &["FOC_DRIVE", "EDDY_PROBE", "TOOLHEAD", "CHAMBER_IO", "POWER", "MMU"], 2, "Required class. The Host refuses to run if the node on the bus differs."),
            pi("short_address", "Short address", 49, "", 1, 127, "Assigned by the Host on first contact."),
            ps("canbus_uuid", "Factory ID", "5A3E04C290AA0041", "64-bit factory ID printed on the board."),
            pc("canbus_interface", "Segment", &["CAN FD A", "CAN FD B"], 1, "Segment the node is wired to."),
            pb("required", "Required", true, "Refuse to start if this node is missing."),
        ]),
        m("temperature_mcu", "MCU temperature", "temperature_mcu {}", 7, true, "Report a node's internal temperature.", &["mcu"], vec![
            pc("sensor_mcu", "Node", NODE_ALL, 0, "Node to read."),
            pf("sensor_temperature1", "Calibration temp", 25.0, "C", 0.0, 100.0, "Calibration reference temperature."),
        ]),
    ]
}

pub struct Builder {
    pub defs: Vec<ModDef>,
    pub blocks: Vec<Block>,
    next: u32,
    pub search: String,
    pub tab: usize,
    pub import_text: String,
    pub import_msg: Vec<String>,
    pub replace: bool,
    pub force_open: Option<bool>,
}

pub fn value_str(kind: &PKind, v: &PVal) -> String {
    match (kind, v) {
        (_, PVal::F(x)) => format!("{}", (x * 1.0e6).round() / 1.0e6),
        (_, PVal::I(x)) => x.to_string(),
        (_, PVal::B(x)) => if *x { "True".into() } else { "False".into() },
        (_, PVal::S(x)) => x.clone(),
        (PKind::C(o), PVal::C(i)) => o.get(*i).copied().unwrap_or("").to_string(),
        _ => String::new(),
    }
}

pub fn invalid(kind: &PKind, v: &PVal) -> Option<String> {
    match (kind, v) {
        (PKind::F { min, max, unit }, PVal::F(x)) if *x < *min || *x > *max => Some(format!("out of range {} .. {} {}", min, max, unit)),
        (PKind::I { min, max, unit }, PVal::I(x)) if *x < *min || *x > *max => Some(format!("out of range {} .. {} {}", min, max, unit)),
        _ => None,
    }
}

impl Builder {
    pub fn new() -> Self {
        let mut b = Builder { defs: defs(), blocks: Vec::new(), next: 1, search: String::new(), tab: 0, import_text: SAMPLE_CFG.into(), import_msg: Vec::new(), replace: true, force_open: None };
        b.starter();
        b
    }

    pub fn def_index(&self, id: &str) -> Option<usize> {
        self.defs.iter().position(|d| d.id == id)
    }

    pub fn add(&mut self, def: usize, name: &str) -> u32 {
        let uid = self.next;
        self.next += 1;
        let vals = self.defs[def].params.iter().map(|p| p.def.clone()).collect();
        self.blocks.push(Block { uid, def, name: name.into(), on: true, vals });
        uid
    }

    pub fn set(&mut self, uid: u32, key: &str, v: PVal) {
        if let Some(b) = self.blocks.iter_mut().find(|b| b.uid == uid) {
            if let Some(i) = self.defs[b.def].params.iter().position(|p| p.key == key) {
                b.vals[i] = v;
            }
        }
    }

    pub fn dup(&mut self, uid: u32) {
        if let Some(b) = self.blocks.iter().find(|b| b.uid == uid).cloned() {
            let mut n = b;
            n.uid = self.next;
            self.next += 1;
            n.name = format!("{}_copy", n.name);
            self.blocks.push(n);
        }
    }

    pub fn reset(&mut self, uid: u32) {
        if let Some(b) = self.blocks.iter_mut().find(|b| b.uid == uid) {
            b.vals = self.defs[b.def].params.iter().map(|p| p.def.clone()).collect();
        }
    }

    pub fn present(&self, id: &str) -> bool {
        self.defs.iter().position(|d| d.id == id).map(|i| self.blocks.iter().any(|b| b.def == i && b.on)).unwrap_or(false)
    }

    pub fn missing(&self, b: &Block) -> Vec<&'static str> {
        self.defs[b.def].requires.iter().copied().filter(|r| !self.present(r)).collect()
    }

    pub fn issues(&self, b: &Block) -> usize {
        self.defs[b.def].params.iter().zip(&b.vals).filter(|(p, v)| invalid(&p.kind, v).is_some()).count() + self.missing(b).len()
    }

    pub fn modified(&self, b: &Block) -> usize {
        self.defs[b.def].params.iter().zip(&b.vals).filter(|(p, v)| p.def != **v).count()
    }

    fn starter(&mut self) {
        let add = |me: &mut Builder, id: &str, name: &str| -> u32 {
            let d = me.def_index(id).unwrap();
            me.add(d, name)
        };
        add(self, "printer", "");
        for (n, node, endstop) in [("x", 0, 0.0), ("y", 1, 0.0), ("z", 2, 0.0), ("z1", 3, 0.0), ("z2", 4, 0.0)] {
            let u = add(self, "stepper", n);
            self.set(u, "node", PVal::C(node));
            self.set(u, "position_endstop", PVal::F(endstop));
            if n.starts_with('z') {
                self.set(u, "rotation_distance", PVal::F(4.0));
                self.set(u, "position_max", PVal::F(330.0));
                self.set(u, "homing_speed", PVal::F(8.0));
                self.set(u, "second_homing_speed", PVal::F(4.0));
                self.set(u, "endstop_mode", PVal::C(2));
            }
        }
        add(self, "extruder_motion", "");
        add(self, "input_shaper", "");
        add(self, "safe_z_home", "");

        add(self, "hotend", "");
        add(self, "heater_bed", "");
        add(self, "heater_generic", "chamber");
        let u = add(self, "temperature_sensor", "mainboard_p4");
        self.set(u, "max_temp", PVal::F(85.0));

        add(self, "fan", "");
        add(self, "heater_fan", "heatsink");
        let u = add(self, "fan_generic", "exhaust");
        self.set(u, "node", PVal::C(1));
        let u = add(self, "fan_generic", "filter");
        self.set(u, "node", PVal::C(1));
        add(self, "controller_fan", "board");

        add(self, "probe", "");
        add(self, "bed_mesh", "");
        add(self, "z_tilt", "");
        add(self, "axis_twist", "");

        add(self, "filament_switch_sensor", "toolhead");
        add(self, "adxl345", "probe");
        add(self, "resonance_tester", "");
        add(self, "mmu", "");

        add(self, "neopixel", "chamber_led");
        add(self, "display", "");

        add(self, "virtual_sdcard", "");
        add(self, "exclude_object", "");
        add(self, "firmware_retraction", "");
        add(self, "idle_timeout", "");
        add(self, "pause_resume", "");
        add(self, "verify_heater", "extruder");
        add(self, "verify_heater", "heater_bed");
        add(self, "power_loss", "");

        let u = add(self, "canbus", "a");
        self.set(u, "segment", PVal::C(0));
        let u = add(self, "canbus", "b");
        self.set(u, "segment", PVal::C(1));
        self.set(u, "data_bitrate", PVal::I(4_000_000));
        let u = add(self, "mcu", "toolhead");
        self.set(u, "short_address", PVal::I(0x31));
        let u = add(self, "mcu", "probe");
        self.set(u, "class", PVal::C(1));
        self.set(u, "short_address", PVal::I(0x30));
    }

    pub fn section_name(&self, b: &Block) -> String {
        let d = &self.defs[b.def];
        if d.fmt.contains("{}") { d.fmt.replace("{}", &b.name) } else { d.fmt.to_string() }
    }

    /// Render all enabled blocks as a config file in Klipper style.
    pub fn generate(&self) -> String {
        let mut out = String::from("# MACH5 printer config - generated from blocks\n");
        let n = self.blocks.iter().filter(|b| b.on).count();
        out.push_str(&format!("# {} modules, schema 1.4.2\n", n));
        // Blocks that share one Klipper section (extruder motion + hotend) are merged into it,
        // emitted once in the first group that mentions it.
        let mut emitted: Vec<String> = Vec::new();
        for (g, (gname, _)) in GROUPS.iter().enumerate() {
            let list: Vec<&Block> = self.blocks.iter().filter(|b| b.on && self.defs[b.def].group == g).collect();
            let mut body = String::new();
            for b in list {
                let sec = self.section_name(b);
                if emitted.contains(&sec) {
                    continue;
                }
                emitted.push(sec.clone());
                body.push_str(&format!("\n[{}]\n", sec));
                for sb in self.blocks.iter().filter(|x| x.on && self.section_name(x) == sec) {
                    for (p, v) in self.defs[sb.def].params.iter().zip(&sb.vals) {
                        let s = value_str(&p.kind, v);
                        if s.is_empty() {
                            continue;
                        }
                        if s.contains('\n') {
                            body.push_str(&format!("{}:\n", p.key));
                            for l in s.lines() {
                                body.push_str(&format!("  {}\n", l));
                            }
                        } else {
                            body.push_str(&format!("{}: {}\n", p.key, s));
                        }
                    }
                }
            }
            if !body.is_empty() {
                out.push_str(&format!("\n#--- {} ---\n{}", gname, body));
            }
        }
        out
    }

    /// Import a Klipper-style config: sections become blocks, known keys become parameters.
    pub fn import(&mut self, text: &str) {
        let mut msg = Vec::new();
        if self.replace {
            self.blocks.clear();
        }
        let mut cur: Vec<u32> = Vec::new();
        let (mut sections, mut keys, mut unmapped) = (0, 0, 0);
        let mut last_key: Option<(String, String)> = None;
        for raw in text.lines() {
            let line = raw.split('#').next().unwrap_or("").trim_end();
            if line.trim().is_empty() {
                continue;
            }
            if let Some(rest) = line.trim().strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                let sec = rest.trim();
                cur.clear();
                last_key = None;
                let mut found = false;
                for di in 0..self.defs.len() {
                    let fmt = self.defs[di].fmt;
                    let name = if let Some(pre) = fmt.strip_suffix("{}") {
                        sec.strip_prefix(pre).filter(|n| !n.is_empty()).map(|n| n.to_string())
                    } else if fmt == sec {
                        Some(String::new())
                    } else {
                        None
                    };
                    if let Some(n) = name {
                        // two modules can share one Klipper section (extruder)
                        let uid = self.add(di, &n);
                        cur.push(uid);
                        found = true;
                    }
                }
                if found { sections += 1; } else { msg.push(format!("section [{}] has no module yet - skipped", sec)); }
                continue;
            }
            if line.starts_with(' ') || line.starts_with('\t') {
                // continuation of a multi-line value
                if let Some((k, _)) = &last_key {
                    let add = line.trim().to_string();
                    let k = k.clone();
                    for &u in &cur {
                        self.append_s(u, &k, &add);
                    }
                }
                continue;
            }
            let (k, v) = match line.split_once(':').or_else(|| line.split_once('=')) {
                Some((k, v)) => (k.trim().to_string(), v.trim().to_string()),
                None => continue,
            };
            last_key = Some((k.clone(), v.clone()));
            let mut done = false;
            for &u in &cur {
                if self.set_parsed(u, &k, &v) {
                    keys += 1;
                    done = true;
                    break;
                }
            }
            if !done && !cur.is_empty() {
                unmapped += 1;
                if unmapped <= 12 {
                    msg.push(format!("unmapped key '{}' (value kept out of the block)", k));
                }
            }
        }
        msg.insert(0, format!("Imported {} sections, {} parameters; {} keys had no matching parameter.", sections, keys, unmapped));
        self.import_msg = msg;
    }

    fn append_s(&mut self, uid: u32, key: &str, add: &str) {
        let di = match self.blocks.iter().find(|b| b.uid == uid) { Some(b) => b.def, None => return };
        if let Some(i) = self.defs[di].params.iter().position(|p| p.key == key) {
            if let Some(b) = self.blocks.iter_mut().find(|b| b.uid == uid) {
                if let PVal::S(s) = &mut b.vals[i] {
                    if !s.is_empty() { s.push('\n'); }
                    s.push_str(add);
                }
            }
        }
    }

    fn set_parsed(&mut self, uid: u32, key: &str, v: &str) -> bool {
        let di = match self.blocks.iter().find(|b| b.uid == uid) { Some(b) => b.def, None => return false };
        let i = match self.defs[di].params.iter().position(|p| p.key == key) { Some(i) => i, None => return false };
        let val = match &self.defs[di].params[i].kind {
            PKind::F { .. } => v.split_whitespace().next().and_then(|s| s.parse::<f64>().ok()).map(PVal::F),
            PKind::I { .. } => v.split_whitespace().next().and_then(|s| s.parse::<i64>().ok()).map(PVal::I),
            PKind::B => Some(PVal::B(matches!(v.to_lowercase().as_str(), "true" | "yes" | "1" | "on"))),
            PKind::S => Some(PVal::S(v.to_string())),
            PKind::C(o) => o.iter().position(|x| x.eq_ignore_ascii_case(v)).map(PVal::C),
        };
        match val {
            Some(x) => {
                if let Some(b) = self.blocks.iter_mut().find(|b| b.uid == uid) {
                    b.vals[i] = x;
                }
                true
            }
            None => false,
        }
    }
}

pub const SAMPLE_CFG: &str = "# Pasted Klipper printer.cfg (excerpt)
[printer]
kinematics: corexy
max_velocity: 400
max_accel: 8000
max_z_velocity: 30

[stepper_x]
step_pin: PF13
dir_pin: PF12
enable_pin: !PF14
rotation_distance: 40
microsteps: 32
full_steps_per_rotation: 200
position_endstop: 300
position_max: 300
homing_speed: 80

[extruder]
nozzle_diameter: 0.400
filament_diameter: 1.75
rotation_distance: 22.6789
pressure_advance: 0.05
sensor_type: ATC Semitec 104GT-2
min_temp: 0
max_temp: 290
control: pid
pid_Kp: 26.2
pid_Ki: 1.9
pid_Kd: 90.1

[heater_bed]
sensor_type: NTC 100K MGB18
max_temp: 110

[heater_fan hotend_fan]
heater: extruder
heater_temp: 50.0
fan_speed: 1.0

[bed_mesh]
speed: 200
horizontal_move_z: 5
mesh_min: 30, 30
mesh_max: 270, 270
probe_count: 7, 7
algorithm: bicubic

[input_shaper]
shaper_freq_x: 55.8
shaper_type_x: mzv
shaper_freq_y: 38.6
shaper_type_y: ei

[tmc2209 stepper_x]
uart_pin: PC4
run_current: 0.8
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_then_import_round_trips() {
        let mut b = Builder::new();
        let text = b.generate();
        assert!(text.contains("[stepper_x]") && text.contains("[bed_mesh]") && text.contains("[heater_fan heatsink]"));
        let n = b.blocks.len();
        b.import(&text);
        assert_eq!(b.blocks.iter().filter(|x| x.on).count() >= n - 2, true, "{:?}", b.import_msg);
        assert_eq!(b.generate().lines().filter(|l| l.starts_with('[')).count(), text.lines().filter(|l| l.starts_with('[')).count());
    }

    #[test]
    fn klipper_sample_maps_known_keys() {
        let mut b = Builder::new();
        b.import(SAMPLE_CFG);
        assert!(b.import_msg[0].contains("Imported"));
        let x = b.blocks.iter().find(|x| b.defs[x.def].id == "stepper" && x.name == "x").unwrap();
        let i = b.defs[x.def].params.iter().position(|p| p.key == "position_max").unwrap();
        assert_eq!(x.vals[i], PVal::F(300.0));
        assert!(b.import_msg.iter().any(|m| m.contains("tmc2209")), "unknown section is reported");
    }
}

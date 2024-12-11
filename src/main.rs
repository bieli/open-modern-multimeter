use clap::{Arg, Command};
use raylib::ffi::LoadFontFromMemory;
use raylib::prelude::*;
use std::ffi::CString;
use std::io::{self, Write};
use std::ptr::null_mut;
use std::time::Duration;
use csv::WriterBuilder;
use std::fs::OpenOptions;
use chrono::prelude::*;

const SCREEN_WIDTH: i32 = 900;
const SCREEN_HEIGHT: i32 = 150;
const UNIT_SCREEN_WIDTH: f32 = 680.0;
const DISPLAY_POS_10: f32 = 10.0;
const DISPLAY_POS_20: f32 = 20.0;
const DISPLAY_FONT_SIZE_140: f32 = 140.0;
const DISPLAY_CHANNEL_COLOR: Color = Color::WHITE;
const DISPLAY_BACKGROUND_COLOR: Color = Color::BLACK;

const SERIAL_BUFFER_SIZE: i32 = 32;
const SERIAL_TIMEOUT_MILISEC: i32 = 10;
const SCPI_MEAS_CMD_OWON: &[u8; 6] = b"MEAS?\n";
const SCPI_MEAS_CMD_AGILENT: &[u8; 14] = b"MEAS:VOLT:DC?\n"; // before SYST:REM
const APP_NAME: &str = "Open Modern Multimeter";

#[derive(Debug)]
pub struct Config {
    port_name: String,
    baud_rate: u32,
    channel_no: u32,
    unit: String,
    window_position: String,
    scpi_protocol_enabled: bool,
    enable_chart: String,
    color: Color,
    enable_csv_logger: bool,
}

impl Config {
    pub fn new(matches: &clap::ArgMatches) -> Result<Self, String> {
        let port_name = matches.value_of("port").unwrap().to_string();
        let baud_rate = matches
            .value_of("baud")
            .unwrap()
            .parse::<u32>()
            .map_err(|_| "Invalid baud rate".to_string())?;
        let channel_no = matches
            .value_of("channel_no")
            .unwrap()
            .parse::<u32>()
            .map_err(|_| "Invalid channel number".to_string())?;
        let unit = matches.value_of("unit").unwrap().to_string();
        let window_position = matches.value_of("window_position").unwrap().to_string();
        let enable_chart = matches.value_of("enable_chart").unwrap().to_string();
        let color = match matches.value_of("color") {
            Some("r") => Color::RED,
            Some("g") => Color::GREEN,
            Some("b") => Color::BLUE,
            _ => Color::RED,
        };
        let scpi_protocol_enabled = match matches.value_of("scpi_protocol_enabled") {
            Some("1") => true,
            Some("0") => false,
            _ => false,
        };
        let enable_csv_logger = match matches.value_of("enable_csv_logger") {
            Some("1") => true,
            Some("0") => false,
            _ => false,
        };

        Ok(Config {
            port_name,
            baud_rate,
            channel_no,
            unit,
            window_position,
            scpi_protocol_enabled,
            enable_chart,
            color,
            enable_csv_logger,
        })
    }

    fn validate_number(val: &str) -> Result<(), String> {
        val.parse::<i32>()
            .map(|_| ())
            .map_err(|_| format!("`{}` is not a valid integer!", val))
    }

    fn valid_baud(val: &str) -> Result<(), String> {
        val.parse::<u32>()
            .map(|_| ())
            .map_err(|_| format!("Invalid baud rate '{}' specified", val))
    }
}

struct Display {
    font: Font,
}

impl Display {
    fn new(font_file: &[u8]) -> Self {
        let font_file_size = font_file.len();
        let font_type = CString::new(".ttf").unwrap();
        let chars = null_mut();
        let font = unsafe {
            Font::from_raw(LoadFontFromMemory(
                font_type.as_ptr(),
                font_file.as_ptr(),
                font_file_size.try_into().unwrap(),
                256,
                chars,
                100,
            ))
        };
        Display { font }
    }

    fn draw(
        &self,
        d: &mut RaylibDrawHandle<'_>,
        channel_no: u32,
        value: &str,
        unit: &str,
        color: &Color,
    ) {
        d.draw_text_ex(
            &self.font,
            &format!("CH:{}", channel_no),
            Vector2::new(DISPLAY_POS_10, DISPLAY_POS_10),
            DISPLAY_POS_20,
            DISPLAY_POS_10,
            DISPLAY_CHANNEL_COLOR,
        );
        d.draw_text_ex(
            &self.font,
            &value,
            Vector2::new(DISPLAY_POS_20 * 2.0, DISPLAY_POS_20),
            DISPLAY_FONT_SIZE_140,
            DISPLAY_POS_10,
            color,
        );
        d.draw_text_ex(
            &self.font,
            &unit,
            Vector2::new(UNIT_SCREEN_WIDTH, DISPLAY_POS_20),
            DISPLAY_FONT_SIZE_140,
            DISPLAY_POS_10,
            color,
        );
    }
}

fn get_screen_resolution() -> (i32, i32) {
    unsafe {
        let monitor_index = raylib::ffi::GetCurrentMonitor();
        let width = raylib::ffi::GetMonitorWidth(monitor_index);
        let height = raylib::ffi::GetMonitorHeight(monitor_index);
        (width, height)
    }
}

fn calculate_window_position(
    position_code: &str,
    screen_width: i32,
    screen_height: i32,
    window_width: i32,
    window_height: i32,
) -> (i32, i32) {
    let parts: Vec<&str> = position_code.split('_').collect();
    if parts.len() != 2 {
        panic!("Invalid position code format. Expected format: 'X_Y'.");
    }

    let horizontal_section: i32 = parts[0].parse().expect("Invalid horizontal section.");
    let vertical_section: i32 = parts[1].parse().expect("Invalid vertical section.");

    if horizontal_section < 1
        || horizontal_section > 4
        || vertical_section < 1
        || vertical_section > 4
    {
        panic!("Position sections must be between 1 and 4.");
    }

    let section_width = screen_width / 4;
    let section_height = screen_height / 4;

    let x = (horizontal_section - 1) * section_width + (section_width - window_width) / 2;
    let y = (vertical_section - 1) * section_height + (section_height - window_height) / 2;

    (x.max(0), y.max(0))
}

/*
fn read_serial_data(port: &mut dyn serialport::SerialPort, serial_buf: &mut Vec<u8>) -> Result<String, String> {
    match port.read(serial_buf.as_mut_slice()) {
        Ok(bytes_read) => {
            io::stdout().write_all(&serial_buf[..bytes_read]).unwrap();
            io::stdout().flush().unwrap();

            // Filter out null bytes and truncate the buffer
            let mut serial_buf_filtered = serial_buf.clone();
            serial_buf_filtered.retain(|&x| x != 0);
            if serial_buf_filtered.len() > 8 {
                serial_buf_filtered.truncate(9);
            }

            // Try to convert to a string
            match CString::new(serial_buf_filtered) {
                Ok(cstr) => cstr.into_string().map_err(|_| "Data conversion error".to_string()),
                Err(_) => Err("Invalid data from serial port".to_string()),
            }
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => Ok("".to_string()),
        Err(e) => Err(format!("Error reading data from port: {:?}", e)),
    }
}
*/

struct Histogram {
    bins: Vec<u32>,   // Frequency count for each bin
    min_value: f32,   // Minimum value represented in the histogram
    max_value: f32,   // Maximum value represented in the histogram
    bin_count: usize, // Number of bins
}

impl Histogram {
    fn new(min_value: f32, max_value: f32, bin_count: usize) -> Self {
        Self {
            bins: vec![0; bin_count],
            min_value,
            max_value,
            bin_count,
        }
    }

    fn add_value(&mut self, value: f32) {
        if value < self.min_value || value > self.max_value {
            return;
        }
        let bin_index = ((value - self.min_value) / (self.max_value - self.min_value)
            * self.bin_count as f32) as usize;
        self.bins[bin_index] += 1;
    }

    /*
        fn reset(&mut self) {
            for bin in &mut self.bins {
                *bin = 0;
            }
        }
    */
    fn normalized_bins(&self) -> Vec<f32> {
        let max_count = *self.bins.iter().max().unwrap_or(&1) as f32;
        self.bins
            .iter()
            .map(|&count| count as f32 / max_count)
            .collect()
    }
}

fn render_histogram(
    d: &mut RaylibDrawHandle<'_>,
    histogram: &Histogram,
    pos_x: i32,
    pos_y: i32,
    screen_width: i32,
    screen_height: i32,
    bar_graph_thinkness: f32,
    bar_color: Color,
) {
    let bin_width = (screen_width as f32 / histogram.bin_count as f32) * bar_graph_thinkness;
    let normalized_bins = histogram.normalized_bins();

    for (i, &bin_height) in normalized_bins.iter().enumerate() {
        let x = i as f32 * bin_width;
        let y = screen_height as f32 - (bin_height * 100.0);
        let bar_height = screen_height as f32 - y;
        d.draw_rectangle(
            pos_x + x as i32,
            pos_y + y as i32,
            bin_width as i32,
            bar_height as i32,
            bar_color,
        );
    }
}

fn draw_chart(
    d: &mut RaylibDrawHandle<'_>,
    chart_pos: Vector2,
    chart_width: i32,
    chart_height: i32,
    x_label: &str,
    y_label: &str,
    data_points: &[(f32, f32)],
    point_circle_size: f32,
    grid_step: f32,
    point_color: Color,
    axis_color: Color,
    grid_color: Color,
    label_color: Color,
) {
    let x_axis_y = chart_pos.y + chart_height as f32;
    let y_axis_x = chart_pos.x;

    d.draw_line(
        y_axis_x as i32,
        x_axis_y as i32,
        (y_axis_x + chart_width as f32) as i32,
        x_axis_y as i32,
        axis_color,
    );

    d.draw_line(
        y_axis_x as i32,
        x_axis_y as i32,
        y_axis_x as i32,
        (x_axis_y - chart_height as f32) as i32,
        axis_color,
    );

    d.draw_text(
        x_label,
        (chart_pos.x + chart_width as f32 / 2.0) as i32,
        (x_axis_y + 10.0) as i32,
        20,
        label_color,
    );

    d.draw_text(
        y_label,
        (y_axis_x - 25.0) as i32,
        (chart_pos.y + chart_height as f32 / 2.0) as i32,
        20,
        label_color,
    );

    let max_x = data_points
        .iter()
        .map(|(x, _)| *x)
        .fold(0.0 / 0.0, f32::max);
    let max_y = data_points
        .iter()
        .map(|(_, y)| *y)
        .fold(0.0 / 0.0, f32::max);
    let scale_x = chart_width as f32 / max_x.max(1.0);
    let scale_y = chart_height as f32 / max_y.max(1.0);

    for &(x, y) in data_points {
        let scaled_x = chart_pos.x + x * scale_x;
        let scaled_y = x_axis_y - y * scale_y;
        d.draw_circle(
            scaled_x as i32,
            scaled_y as i32,
            point_circle_size,
            point_color,
        );
    }

    for i in 1..=((chart_width as f32 / grid_step) as i32) {
        let x = y_axis_x + i as f32 * grid_step;
        d.draw_line(
            x as i32,
            x_axis_y as i32,
            x as i32,
            (x_axis_y - chart_height as f32) as i32,
            grid_color,
        );
    }
    for i in 1..=((chart_height as f32 / grid_step) as i32) {
        let y = x_axis_y - i as f32 * grid_step;
        d.draw_line(
            y_axis_x as i32,
            y as i32,
            (y_axis_x + chart_width as f32) as i32,
            y as i32,
            grid_color,
        );
    }
}

fn join(a: &[u8]) -> String {
    use std::fmt::Write;
    a.iter().fold(String::new(), |mut s, &n| {
        write!(s, "{}", (n as u8) as char).ok();
        s
    })
}

/// Converts a scientific notation value stored in `Vec<u8>` into a float representation in `Vec<u8>`.
fn convert_scientific_to_float(input: &[u8]) -> Vec<u8> {
    // Check for 'E' or 'e' in the input
    if input.contains(&b'E') || input.contains(&b'e') {
        // Attempt to parse the Vec<u8> as a UTF-8 string
        match String::from_utf8(input.to_vec()) {
            Ok(string) => match string.trim().parse::<f32>() {
                Ok(num) => {
                    // Format the parsed number back to Vec<u8>
                    return format!("{:.8}", num).into_bytes();
                }
                Err(e) => {
                    eprintln!("Failed to parse number: {}", e);
                }
            },
            Err(e) => {
                eprintln!("Invalid UTF-8 data: {}", e);
            }
        }
    }
    // If parsing fails or 'E'/'e' is not found, return the input unchanged
    input.to_vec()
}

/// Converts a scientific notation value stored in `Vec<u8>` into a float representation as a `String`.
fn convert_scientific_to_float2(input: &[u8]) -> Result<String, String> {
    // Check for 'E' or 'e' in the input
    if input.contains(&b'E') || input.contains(&b'e') {
        // Attempt to parse the Vec<u8> as a UTF-8 string
        match String::from_utf8(input.to_vec()) {
            Ok(string) => match string.trim().parse::<f32>() {
                Ok(num) => {
                    // Format the parsed number back to a readable string
                    let formatted = format!("{:.8}", num);
                    return Ok(join(&formatted.into_bytes())); // Return owned String
                }
                Err(e) => {
                    return Err(format!("Failed to parse number: {}", e));
                }
            },
            Err(e) => {
                return Err(format!("Invalid UTF-8 data: {}", e));
            }
        }
    }
    // If parsing fails or 'E'/'e' is not found, return the original as a String
    Ok(join2(input))
}

/// Joins a byte array into a readable string format.
fn join2(a: &[u8]) -> String {
    use std::fmt::Write;
    a.iter().fold(String::new(), |mut s, &n| {
        write!(s, "{}", n as char).ok();
        s
    })
}


fn append_to_csv(file_path: &str, timestamp: i64, measurement: f32) -> Result<(), Box<dyn std::error::Error>> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)?;
    let mut wtr = WriterBuilder::new()
        .has_headers(false)
        .from_writer(file);
    wtr.write_record(&[timestamp.to_string(), measurement.to_string()])?;
    wtr.flush()?;
    
    Ok(())
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let now = Utc::now().timestamp_millis();
    let matches = Command::new(APP_NAME)
        .about(
            "Reads values from an external multimeter via a serial port and displays measurement values in real-time in a UI",
        )
        .author("code base: https://github.com/bieli/open-modern-multimeter")
        .disable_version_flag(true)
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .help("The device path to the serial port")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("baud")
                .short('b')
                .long("baud")
                .help("The baud rate for communication")
                .takes_value(true)
                .required(true)
                .validator(Config::valid_baud),
        )
        .arg(
            Arg::new("channel_no")
                .short('n')
                .long("channel_no")
                .help("The channel number to display")
                .takes_value(true)
                .required(true)
                .validator(Config::validate_number),
        )
        .arg(
            Arg::new("unit")
                .short('u')
                .long("unit")
                .help("The unit of measurement")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("window_position")
                .short('w')
                .long("window_position")
                .help("Setting up program window position on the screen <x_pos>_<y_pos>, where x_pos and y_pos are in range {1..4} (i.e. 3_3 in the middle of the screen)")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("scpi_protocol_enabled")
                .short('s')
                .long("scpi_protocol_enabled")
                .help("Setting up SCPI protocol for reading measurements from all laboratory multimeters (SCPI 'MEAS?' command send and parse response as measurement value; possible scentific representation of value)")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::new("enable_chart")
                .short('e')
                .long("enable_chart")
                .help("Enable dynamic charts (h: histogram, l: linear) on bottom side of measurement screen.")
                .required(false)
                .default_value(""),
        )
        .arg(
            Arg::new("color")
                .short('c')
                .long("color")
                .help("Color of the display values: r for red, g for green, b for blue (default color is red if not specified)")
                .required(false)
                .default_value("r"),
        )
        .arg(
            Arg::new("enable_csv_logger")
                .short('l')
                .long("enable_csv_logger")
                .help("Enable measurements logger data appender from every value presented in app. on display.")
                .required(false)
                .default_value(""),
        )
        .get_matches();

    let config = Config::new(&matches).unwrap();

    let port = serialport::new(&config.port_name, config.baud_rate)
        .timeout(Duration::from_millis(
            SERIAL_TIMEOUT_MILISEC.try_into().unwrap(),
        ))
        .open();

    let mut screen_height_size = SCREEN_HEIGHT;
    if &config.enable_chart != "" {
        screen_height_size = SCREEN_HEIGHT * 2;
    }

    let (mut rl, thread) = raylib::init()
        .size(SCREEN_WIDTH, screen_height_size)
        .title(&APP_NAME)
        .vsync()
        .build();

    let (max_screen_width, max_screen_height) = get_screen_resolution();
    let (window_pos_x, window_pos_y) = calculate_window_position(
        &config.window_position,
        max_screen_width,
        max_screen_height,
        SCREEN_WIDTH,
        SCREEN_HEIGHT,
    );

    unsafe {
        raylib::ffi::SetWindowPosition(window_pos_x, window_pos_y);
    }

    rl.set_target_fps(60);

    let font_file: &[u8] = include_bytes!("./7_Segment.ttf");
    let display = Display::new(font_file);

    // Adjust min, max, and bin_count as needed
    let mut histogram = Histogram::new(0.0, 10.0, 50);

    let mut data_points = vec![];
    
    let csv_logger_file_name = format!("measurements_{}_{}.csv", now, config.channel_no);

    match port {
        Ok(mut port) => {
            //let mut serial_buf: Vec<u8> = vec![0; SERIAL_BUFFER_SIZE.try_into().unwrap()];
            let mut ts: f32 = 0.0;
            while !rl.window_should_close() {
                let mut serial_buf: Vec<u8> = vec![0; SERIAL_BUFFER_SIZE.try_into().unwrap()];
                //serial_buf = [0; 0].to_vec();
                ts += 1.0;
                if config.scpi_protocol_enabled == true {
                    match port.write(SCPI_MEAS_CMD_OWON) {
                        Ok(_) => {}
                        Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => (),
                        Err(e) => {
                            eprintln!("Error writing data to port (after SCPI cmd.): {:?}", e)
                        }
                    }
                }

                let mut bytes_read_val = 0;
                match port.read(serial_buf.as_mut_slice()) {
                    Ok(bytes_read) => {
                        bytes_read_val = bytes_read;
                        io::stdout().write_all(&serial_buf[..bytes_read]).unwrap();
                        io::stdout().flush().unwrap();
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => (),
                    Err(e) => eprintln!("Error reading data from port: {:?}", e),
                }

                fn extract_first_part(input: &[u8]) -> Vec<u8> {
                    if let Some(position) = input.windows(2).position(|window| window == b"\n") {
                        input[..position].to_vec()
                    } else {
                        input.to_vec() // Return the full buffer if no "\r\n" is found
                    }
                }

                //serial_buf = (&serial_buf[..bytes_read_val]).to_vec();
                //if (serial_buf.is_empty()) {
                //  continue;
                //}
                //serial_buf = b"0.123E-03\r\n".to_vec();
                //serial_buf = b"1.011038E-01\n".to_vec();
                //serial_buf = b"1.013807E-01\r\n1.013807E-01\r\n3807E-01\r\n1.013807E-01\r\n\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0".to_vec();
                //println!("serial_buf: {:#?}", join2(&serial_buf));
                //serial_buf = extract_first_part(&serial_buf);
                //println!("serial_buf: {:#?}", serial_buf);

                let mut serial_buf_filtered = serial_buf.clone();
                serial_buf_filtered.retain(|&x| x != 0);

                //let serial_buf_filtered = convert_scientific_to_float(&serial_buf_filtered);

                //println!("TO: {:?}", join(&convert_scientific_to_float(&serial_buf_filtered)));

                // Convert scientific value
                match convert_scientific_to_float2(&serial_buf_filtered) {
                    Ok(result) => {
                        //println!("TO: {}", result);
                        serial_buf_filtered = result.into();
                    }
                    Err(e) => {
                        eprintln!("Error: {:?}", e);
                        continue;
                    }
                }

                if serial_buf_filtered.len() > 8 {
                    serial_buf_filtered.truncate(9);
                }

                let value = match CString::new(serial_buf_filtered) {
                    Ok(cstr) => match cstr.into_string() {
                        Ok(string) => string,
                        Err(_) => "Data From Serial Port Conversion Error".to_string(),
                    },
                    Err(_) => "Invalid Data From Serial Port".to_string(),
                };
                let mut value_as_f32: f32 = 0.0;
                match value.parse::<f32>() {
                    Ok(value_as_f32_tmp) => {
                        value_as_f32 = value_as_f32_tmp;
                        histogram.add_value(value_as_f32);
                        data_points.push((ts, value_as_f32));
                    }
                    Err(e) => {
                        eprintln!("Error parsing string to f32: {}", e);
                    }
                }
                let mut d = rl.begin_drawing(&thread);
                d.clear_background(DISPLAY_BACKGROUND_COLOR);
                display.draw(
                    &mut d,
                    config.channel_no,
                    &value,
                    &config.unit,
                    &config.color,
                );

                if config.enable_csv_logger && value_as_f32 != 0.0 {
                  let now = Utc::now().timestamp_millis();
                  append_to_csv(&csv_logger_file_name, now, value_as_f32)?;
                }

                if &config.enable_chart == "h" {
                    render_histogram(
                        &mut d,
                        &histogram,
                        (SCREEN_WIDTH / 2) as i32 - 50,
                        -30,
                        SCREEN_WIDTH,
                        screen_height_size,
                        0.3,
                        Color::DARKGRAY,
                    );
                }

                if &config.enable_chart == "l" {
                    draw_chart(
                        &mut d,
                        Vector2::new(40.0, 150.0),
                        SCREEN_WIDTH,
                        100,
                        "T [ms]",
                        "V",
                        &data_points,
                        2.0,
                        50.0,
                        Color::RED,
                        Color::GRAY,
                        Color::DARKGRAY,
                        Color::GRAY,
                    );
                }

                /*
                                //TODO: NEED FIX: below refactoring have blinking effect on screen, so it's known bug ... waiting for hero! ;-)
                                match read_serial_data(&mut *port, &mut serial_buf) {
                                    Ok(value) => {
                                        display.draw(&mut rl, &thread, config.channel_no, &value, &config.unit, &config.color);
                                    }
                                    Err(e) => {
                                        eprintln!("{}", e);
                                    }
                                }
                */
            }
        }
        Err(e) => {
            eprintln!("Failed to open serial port \"{}\": {}", config.port_name, e);
            std::process::exit(1);
        }
    }
    Ok(())
}

/*

// working example of scientific to f32 conversion

use std::fmt::Write;

const SERIAL_BUFFER_SIZE: i32 = 1000;

fn main() {
    println!("{}", join(&vec![1, 2, 3]));

    println!("Scientific values conversion in RUST!");

    let serial_buf = b"0.123E-03\r\n".to_vec(); // Initialize directly
    println!("FROM: {}", join(&serial_buf));

    // Convert scientific value
    match convert_scientific_to_float(&serial_buf) {
        Ok(result) => println!("TO: {}", result),
        Err(e) => eprintln!("Error: {}", e),
    }
    println!("END.")
}

/// Converts a scientific notation value stored in `Vec<u8>` into a float representation as a `String`.
fn convert_scientific_to_float(input: &[u8]) -> Result<String, String> {
    // Check for 'E' or 'e' in the input
    if input.contains(&b'E') || input.contains(&b'e') {
        // Attempt to parse the Vec<u8> as a UTF-8 string
        match String::from_utf8(input.to_vec()) {
            Ok(string) => match string.trim().parse::<f32>() {
                Ok(num) => {
                    // Format the parsed number back to a readable string
                    let formatted = format!("{:.8}", num);
                    return Ok(join(&formatted.into_bytes())); // Return owned String
                }
                Err(e) => {
                    return Err(format!("Failed to parse number: {}", e));
                }
            },
            Err(e) => {
                return Err(format!("Invalid UTF-8 data: {}", e));
            }
        }
    }
    // If parsing fails or 'E'/'e' is not found, return the original as a String
    Ok(join(input))
}

/// Joins a byte array into a readable string format.
fn join(a: &[u8]) -> String {
    a.iter().fold(String::new(), |mut s, &n| {
        write!(s, "{}", n as char).ok();
        s
    })
}

*/

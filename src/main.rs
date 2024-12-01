use clap::{Arg, Command};
use raylib::ffi::LoadFontFromMemory;
use raylib::prelude::*;
use std::ffi::CString;
use std::io::{self, Write};
use std::ptr::null_mut;
use std::time::Duration;

const SCREEN_WIDTH: i32 = 900;
const SCREEN_HEIGHT: i32 = 150;
const UNIT_SCREEN_WIDTH: f32 = 680.0;
const DISPLAY_POS_10: f32 = 10.0;
const DISPLAY_POS_20: f32 = 20.0;
const DISPLAY_FONT_SIZE_140: f32 = 140.0;
const DISPLAY_CHANNEL_COLOR: Color = Color::WHITE;
const DISPLAY_BACKGROUND_COLOR: Color = Color::BLACK;

const SERIAL_BUFFER_SIZE: i32 = 1000;
const SERIAL_TIMEOUT_MILISEC: i32 = 10;

const APP_NAME: &str = "Open Modern Multimeter";

struct Config {
    port_name: String,
    baud_rate: u32,
    channel_no: u32,
    unit: String,
    window_position: String,
    enable_chart: String,
    color: Color,
}

impl Config {
    fn new(matches: &clap::ArgMatches) -> Result<Self, String> {
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

        Ok(Config {
            port_name,
            baud_rate,
            channel_no,
            unit,
            window_position,
            enable_chart,
            color,
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

fn main() {
    let matches = Command::new(APP_NAME)
        .about(
            "Reads values from an external multimeter via a serial port and displays measurement values in real-time in a UI",
        )
        .disable_version_flag(true)
        .arg(
            Arg::new("port")
                .help("The device path to the serial port")
                .required(true),
        )
        .arg(
            Arg::new("baud")
                .help("The baud rate for communication")
                .required(true)
                .validator(Config::valid_baud),
        )
        .arg(
            Arg::new("channel_no")
                .help("The channel number to display")
                .required(true)
                .validator(Config::validate_number),
        )
        .arg(
            Arg::new("unit")
                .help("The unit of measurement")
                .required(true),
        )
        .arg(
            Arg::new("window_position")
                .help("Setting up program window position on the screen <x_pos>_<y_pos>, where x_pos and y_pos are in range {1..4} (i.e. 3_3 in the middle of the screen)")
                .required(true),
        )
        .arg(
            Arg::new("enable_chart")
                .help("Enable dynamic charts (h: histogram, l: linear) on bottom side of measurement screen.")
                .required(false)
                .default_value(""),
        )
        .arg(
            Arg::new("color")
                .help("Color of the display values: r for red, g for green, b for blue (default color is red if not specified)")
                .required(false)
                .default_value("r"),
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

    match port {
        Ok(mut port) => {
            let mut serial_buf: Vec<u8> = vec![0; SERIAL_BUFFER_SIZE.try_into().unwrap()];
            let mut ts: f32 = 0.0;
            while !rl.window_should_close() {
                ts += 1.0;
                match port.read(serial_buf.as_mut_slice()) {
                    Ok(bytes_read) => {
                        io::stdout().write_all(&serial_buf[..bytes_read]).unwrap();
                        io::stdout().flush().unwrap();
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => (),
                    Err(e) => eprintln!("Error reading data from port: {:?}", e),
                }

                let mut serial_buf_filtered = serial_buf.clone();
                serial_buf_filtered.retain(|&x| x != 0);

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
                match value.parse::<f32>() {
                    Ok(value_as_f32) => {
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

                if &config.enable_chart == "h" {
                    render_histogram(
                        &mut d,
                        &histogram,
                        (SCREEN_WIDTH / 2) as i32 - 50,
                        -30,
                        SCREEN_WIDTH,
                        screen_height_size,
                        0.3,
                        Color::DARKGRAY, //config.color
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
}

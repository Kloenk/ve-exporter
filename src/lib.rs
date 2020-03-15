use std::error::Error;
use std::fmt;
use std::sync::{Arc, Mutex, RwLock};

use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer};
#[macro_use]
extern crate serde_json;
use serde::Serialize;
use handlebars::Handlebars;
use std::io::{Read, BufReader, BufRead};
use std::ops::Index;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio_serial::{Serial, SerialPortSettings};

//#[macro_use]
//extern crate bitflags;
//pub mod ve;

#[macro_use]
extern crate log;

#[derive(Debug, serde::Serialize)]
pub struct Config {
    pub port: u16,
    pub address: String,
    pub timeout: std::time::Duration,
    pub device: String,
    pub baud_rate: u32,
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(unused_mut)]
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let address = self.address.clone();
        let port = self.port;

        let path = self.device;
        let baud_rate = self.baud_rate;

        let data = Cache::new()?;
        let data_w = data.clone();

        let timeout = self.timeout;

        actix_rt::spawn(async move {
            let data = data_w;

            let mut settings = SerialPortSettings::default();
            settings.baud_rate = baud_rate;
            settings.timeout = timeout;

            // TODO: init serial port
            let mut port = Serial::from_path(path, &settings).unwrap(); // FIXME: unwrap?

            loop {
                info!("run scraper");

                data.update_cache(timeout, &mut port).await;


                // TODO: Remove
                let cache = data.cache.read().unwrap();
                println!("{}", serde_json::to_string(&*cache).unwrap());

                std::thread::sleep(std::time::Duration::from_secs(5)); // TODO: config???
            }
        });

        // handlebars
        let mut handlebars = Handlebars::new();
        handlebars.register_templates_directory(".hb", "./static/templates").unwrap();
        let handlebars_ref = web::Data::new(handlebars);

        HttpServer::new(move || {
            //let data = data.new_2();
            App::new()
                // add data
                .data(data.new_2())
                .app_data(handlebars_ref.clone())
                // enable logger
                .wrap(middleware::Logger::default())
                // enable compression
                .wrap(middleware::Compress::default())
                .service(web::resource("/").to(index))
                .service(web::resource("/index.html").to(index))
                .service(web::resource("/metrics").to(metric))
                .service(web::resource("/metric").to(metric))
        })
        .bind(&format!("{}:{}", address, port))?
        .run()
        .await?;

        Ok(())
    }
}

async fn index() -> HttpResponse {
    HttpResponse::Ok().body(format!(
        include_str!("index.html"),
        env!("CARGO_PKG_VERSION")
    ))
}

async fn metric(
    cache: web::Data<Cache>,
    hb: web::Data<Handlebars>,
    _req: HttpRequest,
) -> HttpResponse {
    let cache = cache.cache.read().unwrap();

    let data = json!({
      "up": if cache.online { "1" } else { "0" }
    });

    let body = hb.render("metric", &data).unwrap();
    /*let ret = format!(
        "# HELP ve_up 1 if the connection is up
# TYPE ve_up gauge
ve_up{{}} {}
",
        if cache.online { "1" } else { "0" }
    );*/

    drop(cache);

    HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4")
        .body(
            /*"# HELP ccsload_up 1 if the instance is up
# TYPE ccsload_up gauge
{}
ccsload_up_sum{{}} {}

# HELP ccsload_load the heap usage of ccsload
# TYPE ccsload_load histogram
{}

# HELP ccsload_process threads used by ccsload
# TYPE ccsload_process gauge
{}

# HELP ccsload_uptime threads used by ccsload
# TYPE ccsload_uptime gauge
{}
            ",
            up, up_sum, load, threads, uptime */
            body
        )
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: 9701u16,
            address: "localhost".to_string(),
            timeout: std::time::Duration::from_secs(5),
            device: "/dev/ttyUSB0".to_string(),
            baud_rate: 19200,
        }
    }
}

#[derive(Debug, Clone)]
struct Cache {
    #[cfg(not(feature = "redis"))]
    cache: Arc<RwLock<CacheData>>,
}

impl Cache {
    #[cfg(not(feature = "redis"))]
    fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            cache: Arc::new(RwLock::new(CacheData::offline()))
        })
    }

    #[cfg(not(feature = "redis"))]
    fn new_2(&self) -> Self {
        Self {
            cache: self.cache.clone(),
        }
    }

    #[cfg(not(feature = "redis"))]
    async fn update_cache(&self, timeout: std::time::Duration, serial: &mut Serial) {
        let mut reader = BufReader::new(serial);

        let mut cachData = CacheData::offline();
        for line in reader.lines() {
            if let Ok(line) = line {
                let val: &str = &line[(line.find("\t").unwrap_or(0))..];
                let val = val.trim();
                if line.starts_with("PID") {
                   cachData.pid = parse_pid(val);
                } else if line.starts_with("IL") {
                    cachData.loadCurrent = val.parse().unwrap_or(0);
                } else if line.starts_with("LOAD") {
                    if val == "ON" {
                        cachData.load = true;
                    } else if val == "OFF" {
                        cachData.load = false;
                    }
                } else if line.starts_with("T") {
                    cachData.temp = val.parse().unwrap_or(0);
                } else if line.starts_with("P") {
                    cachData.power = val.parse().unwrap_or(0);
                } else if line.starts_with("CE") {
                    cachData.consumedAmpHours = val.parse().unwrap_or(0);
                } else if line.starts_with("SOC") {
                    cachData.stateOfCharge = val.parse().unwrap_or(0);
                } else if line.starts_with("TTG") {
                    cachData.timeToGo = val.parse().unwrap_or(0);
                } else if line.starts_with("Alarm") {
                    warn!("ALARM not implemented");
                    //cachData.alarm = true; // FIXME implement
                } else if line.starts_with("Relay") {
                    warn!("RELAY not implemented");
                    //cachData.relay = true;
                } else if line.starts_with("AR") {
                    cachData.alarmReason = val.to_string();
                } else if line.starts_with("OR") {
                    cachData.offReason = val.to_string();
                } else if line.starts_with("H1") {
                    cachData.depthOfDischarge = val.parse().unwrap_or(0);
                } else if line.starts_with("H2") {
                    cachData.lastDischargeDepth = val.parse().unwrap_or(0);
                } else if line.starts_with("H3") {
                    cachData.avgDischargeDepth = val.parse().unwrap_or(0);
                } else if line.starts_with("H4") {
                    cachData.chargeCycles = val.parse().unwrap_or(0);
                } else if line.starts_with("H5") {
                    cachData.discharges = val.parse().unwrap_or(0);
                } else if line.starts_with("H6") {
                    cachData.cumulativeDrawn = val.parse().unwrap_or(0);
                } else if line.starts_with("H7") {
                    cachData.minVoltage = val.parse().unwrap_or(0);
                } else if line.starts_with("H8") {
                    cachData.maxVoltage = val.parse().unwrap_or(0);
                } // TODO: ...
            }
        }






        let mut cache = self.cache.write().unwrap();
        *cache = cachData;
    }

    /*#[cfg(not(feature = "redis"))]
    async fn get(&self, server: &Server) -> CacheData {
        let cache = self.cache.lock().unwrap();
        cache.get(server).unwrap_or(&CacheData::offline()).clone()
    }*/
}

#[derive(Debug, Clone, Serialize)]
struct CacheData {
    online: bool,
    pid: String,

    /// Load Current
    /// Units: mA
    loadCurrent: isize,

    /// Load output state (ON/OFF)
    load: bool,

    /// Battery temperature
    /// Units: C
    temp: isize,

    /// Instantaneous power
    /// Units: W
    power: isize,

    /// Consumed Amp Hours
    /// Units mAh
    consumedAmpHours: isize,

    /// Sate of charge
    /// Unit: Per Mille
    stateOfCharge: isize,

    /// Time to Go
    /// Units: Minutes
    timeToGo: isize,

    /// Alarm condition active
    alarm: bool,

    /// Relay state
    relay: bool,

    /// alarm reason
    /// Units: String?
    alarmReason: String,

    /// Off Reason
    /// Units: String
    offReason: String,

    /// Depth of the deepest discharge
    /// Units: mAh
    depthOfDischarge: isize,

    /// Depth of the last discharge
    /// Units: mAh
    lastDischargeDepth: isize,

    /// Depth of the avarage discharge
    /// Units: mAh
    avgDischargeDepth: isize,

    /// Number of charge Cycles
    chargeCycles: usize,

    /// Number of full discharge
    discharges: usize,

    /// Cumulative Amp Hours Drawn
    /// Units: mAh
    cumulativeDrawn: isize,

    /// Minimum main (battery) voltage
    /// Units: mV
    minVoltage: isize,

    /// Maximum main (battery) voltage
    /// Units: mV
    maxVoltage: isize,

    // ToDo: H9-H18

    /// Yield total (user resettable counter)
    /// Units: 0.01kWh
    yieldTotalUser: usize,

    /// Yield total
    /// Units: 0.01kWh
    yieldTotal: usize,

    /// Maximum Power today
    /// Units: W
    maxPowerToday: usize,

    /// Yield Yesterday
    /// Units: 0.01 kWh
    yieldYesterday: usize,

    /// Maximum Power Yesterday
    /// Units: W
    maxPowerYesterday: usize,

    /// Error Code
    /// Units: String?
    errCode: String,

    /// State of operation
    ///
    /// # States
    /// Off | 0
    /// Low power | 1
    /// Fault | 2
    /// Bulk | 3
    /// Absorption | 4
    /// Float | 5
    /// Storage | 6
    /// Equalize (manual) | 7
    /// Inverting | 9
    /// Power supply | 11
    /// Starting-up | 245
    /// Repeated absorption | 246
    /// Auto equalize / Recondition | 247
    /// BatterySafe | 248
    /// External Control | 252
    state: u8,

    /// Firmware version (16Bit)
    firmware16: u16,

    /// Firmware version (24Bit)
    firmware24: u32,

    /// Serial number
    serialNumber: String,

    /// Day sequence number (0..364)
    day: u16,

    /// AC output voltage
    /// Units: 0.01 V
    ACOut: usize,

    /// AC output current
    /// Units: 0.01 A
    ACCurrent: usize,

    /// AC output apparent power
    /// Units: VA
    ACVA: usize,

    // TODO: implement WARN, MPPT
}

impl CacheData {
    fn offline() -> Self {
        Self {
            online: false,
            pid: "unknown".to_string(),
            loadCurrent: 0,
            load: false,
            temp: 0,
            power: 0,
            consumedAmpHours: 0,
            stateOfCharge: 0,
            timeToGo: 0,
            alarm: false,
            relay: false,
            alarmReason: "".to_string(),
            offReason: "".to_string(),
            depthOfDischarge: 0,
            lastDischargeDepth: 0,
            avgDischargeDepth: 0,
            chargeCycles: 0,
            discharges: 0,
            cumulativeDrawn: 0,
            minVoltage: 0,
            maxVoltage: 0,
            yieldTotalUser: 0,
            yieldTotal: 0,
            maxPowerToday: 0,
            yieldYesterday: 0,
            maxPowerYesterday: 0,
            errCode: "".to_string(),
            state: 0,
            firmware16: 0,
            firmware24: 0,
            serialNumber: "".to_string(),
            day: 0,
            ACOut: 0,
            ACCurrent: 0,
            ACVA: 0,
        }
    }
}

fn parse_pid(pid: &str) -> String {
    match pid {
        "0x203" => "BMV-700",
        "0x204" => "BMV-702",

        _ => "Unknown"
    }.to_string()
}

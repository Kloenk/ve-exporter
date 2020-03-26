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
use futures::{SinkExt, StreamExt};
use serde_json::Value;

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

            let client = reqwest::ClientBuilder::new()
                .timeout(timeout)
                .user_agent(format!("ve/monitoring ({})", env!("CARGO_PKG_VERSION")))
                .use_rustls_tls()
                .build()
                .unwrap();

            loop {
                info!("run scraper");

                let mut rCache = data.cache.write().unwrap();
                *rCache = match Cache::update_cache(&path, &client).await {
                    Ok(v) => v,
                    Err(e) => { trace!("error updating: {:?}", e); CacheData::offline() },
                };
                drop(rCache);


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

    let data = cache.clone();
    //let data = json!{"cache": data};

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
    async fn update_cache(address: &str, client: &reqwest::Client) -> Result<CacheData, Box<dyn std::error::Error>> {
        let resp = client
            .get(address)
            .send()
            .await?;

        let resp = resp.text().await?;
        let resp: Value = serde_json::from_str(&resp)?;

        let mut cachData = CacheData::offline();
        cachData.online = true;
        trace!("resp: {:?}", resp);
        cachData.voltageCurrent = resp.get("V").map(|v| v.as_str().map(|v| v.parse().ok())).flatten().flatten().unwrap_or(0);
        cachData.serialNumber = resp.get("SER#").map(|v| v.as_str()).flatten().unwrap_or("").to_string();
        cachData.state = resp.get("CS").map(|v| v.as_str().map(|v| v.parse::<u8>().ok())).flatten().flatten().unwrap_or(0);
        cachData.current = resp.get("I").map(|v| v.as_str().map(|v| v.parse::<isize>().ok())).flatten().flatten().unwrap_or(0);
        cachData.yieldTotalUser = resp.get("H19").map(|v| v.as_str().map(|v| v.parse::<usize>().ok())).flatten().flatten().unwrap_or(0);
        cachData.day = resp.get("HSDS").map(|v| v.as_str().map(|v| v.parse::<u16>().ok())).flatten().flatten().unwrap_or(0);
        cachData.yieldTotal = resp.get("H20").map(|v| v.as_str().map(|v| v.parse::<usize>().ok())).flatten().flatten().unwrap_or(0);
        cachData.maxPowerYesterday = resp.get("H23").map(|v| v.as_str().map(|v| v.parse::<usize>().ok())).flatten().flatten().unwrap_or(0);
        cachData.load = resp.get("LOAD").map(|v| v.as_str().map(|v| match v {
            "ON" => true,
            _ => false,
        })).flatten().unwrap_or(false);
        cachData.pannelPower = resp.get("PPV").map(|v| v.as_str().map(|v| v.parse::<isize>().ok())).flatten().flatten().unwrap_or(0);
        cachData.loadCurrent = resp.get("IL").map(|v| v.as_str().map(|v| v.parse::<isize>().ok())).flatten().flatten().unwrap_or(0);
        cachData.panelVoltage = resp.get("VPV").map(|v| v.as_str().map(|v| v.parse::<isize>().ok())).flatten().flatten().unwrap_or(0);
        cachData.yieldYesterday = resp.get("H22").map(|v| v.as_str().map(|v| v.parse::<usize>().ok())).flatten().flatten().unwrap_or(0);
        cachData.firmware16 = resp.get("FW").map(|v| v.as_str().map(|v| v.parse::<u16>().ok())).flatten().flatten().unwrap_or(0);
        cachData.maxPowerToday = resp.get("H21").map(|v| v.as_str().map(|v| v.parse::<usize>().ok())).flatten().flatten().unwrap_or(0);
        cachData.offReason = resp.get("OR").map(|v| v.as_str()).flatten().unwrap_or("").to_string();
        cachData.errCode = resp.get("ERR").map(|v| v.as_str()).flatten().unwrap_or("").to_string();
        cachData.pid = resp.get("PID").map(|v| v.as_str().map(|v| parse_pid(v))).flatten().unwrap_or("".to_string());
        Ok(cachData)
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

    /// voltage current
    /// Units: mV
    voltageCurrent: isize,

    /// Load Current
    /// Units: mA
    loadCurrent: isize,

    /// Main or channel 1 battery current
    /// Units: mA
    current: isize,

    /// Panel voltage
    /// Units: mV
    panelVoltage: isize,

    /// Panel Power
    /// Units: W
    pannelPower: isize,

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
            voltageCurrent: 0,
            loadCurrent: 0,
            current: 0,
            panelVoltage: 0,
            pannelPower: 0,
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
        "0x205" => "BMV-700H",
        "0x0300" => "BlueSolar MPPT 70|15",
        "0xA040" => "BlueSolar MPPT 75|50",
        "0xA041" => "BlueSolar MPPT 150|35",
        "0xA042" => "BlueSolar MPPT 75|15",
        "0xA043" => "BlueSolar MPPT 100|15",
        "0xA044" => "BlueSolar MPPT 100|30",
        "0xA045" => "BlueSolar MPPT 100|50",
        "0xA046" => "BlueSolar MPPT 150|70",
        "0xA047" => "BlueSolar MPPT 150|100",
        "0xA049" => "BlueSolar MPPT 100|50 rev2",
        "0xA04A" => "BlueSolar MPPT 100|30 rev2",
        "0xA04B" => "BlueSolar MPPT 150|35 rev2",
        "0xA04C" => "BlueSolar MPPT 75|10",
        "0xA04D" => "BlueSolar MPPT 150|45",
        "0xA04E" => "BlueSolar MPPT 150|60",
        "0xA04F" => "BlueSolar MPPT 150|85",
        "0xA050" => "SmartSolar MPPT 250|100",
        "0xA051" => "SmartSolar MPPT 150|100",
        "0xA052" => "SmartSolar MPPT 150|85",
        "0xA053" => "SmartSolar MPPT 75|15",
        "0xA054" => "SmartSolar MPPT 75|10",
        "0xA055" => "SmartSolar MPPT 100|15",
        "0xA056" => "SmartSolar MPPT 100|30",
        "0xA057" => "SmartSolar MPPT 100|50",
        "0xA058" => "SmartSolar MPPT 150|35",
        "0xA059" => "SmartSolar MPPT 150|100 rev2",
        "0xA05A" => "SmartSolar MPPT 150|85 rev2",
        "0xA05B" => "SmartSolar MPPT 250|70",
        "0xA05C" => "SmartSolar MPPT 250|85",
        "0xA05D" => "SmartSolar MPPT 250|60",
        "0xA05E" => "SmartSolar MPPT 250|45",
        "0xA05F" => "SmartSolar MPPT 100|20",
        "0xA060" => "SmartSolar MPPT 100|20 48V",
        "0xA061" => "SmartSolar MPPT 150|45",
        "0xA062" => "SmartSolar MPPT 150|60",
        "0xA063" => "SmartSolar MPPT 150|70",
        "0xA064" => "SmartSolar MPPT 250|85 rev2",
        "0xA065" => "SmartSolar MPPT 250|100 rev2",
        "0xA102" => "SmartSolar MPPT VE.Can 150/70",
        "0xA103" => "SmartSolar MPPT VE.Can 150/45",
        "0xA104" => "SmartSolar MPPT VE.Can 150/60",
        "0xA105" => "SmartSolar MPPT VE.Can 150/85",
        "0xA106" => "SmartSolar MPPT VE.Can 150/100",
        "0xA107" => "SmartSolar MPPT VE.Can 250/45",
        "0xA108" => "SmartSolar MPPT VE.Can 250/60",
        "0xA109" => "SmartSolar MPPT VE.Can 250/70",
        "0xA10A" => "SmartSolar MPPT VE.Can 250/85",
        "0xA10B" => "SmartSolar MPPT VE.Can 250/100",
        "0xA201" => "Phoenix Inverter 12V 250VA 230V",
        "0xA202" => "Phoenix Inverter 24V 250VA 230V",
        "0xA204" => "Phoenix Inverter 48V 250VA 230V",
        "0xA211" => "Phoenix Inverter 12V 375VA 230V",
        "0xA212" => "Phoenix Inverter 24V 375VA 230V",
        "0xA214" => "Phoenix Inverter 48V 375VA 230V",
        "0xA221" => "Phoenix Inverter 12V 500VA 230V",
        "0xA222" => "Phoenix Inverter 24V 500VA 230V",
        "0xA224" => "Phoenix Inverter 48V 500VA 230V",
        "0xA231" => "Phoenix Inverter 12V 250VA 230V",
        "0xA232" => "Phoenix Inverter 24V 250VA 230V",
        "0xA234" => "Phoenix Inverter 48V 250VA 230V",
        "0xA239" => "Phoenix Inverter 12V 250VA 120V",
        "0xA23A" => "Phoenix Inverter 24V 250VA 120V",
        "0xA23C" => "Phoenix Inverter 48V 250VA 120V",
        "0xA241" => "Phoenix Inverter 12V 375VA 230V",
        "0xA242" => "Phoenix Inverter 24V 375VA 230V",
        "0xA244" => "Phoenix Inverter 48V 375VA 230V",
        "0xA249" => "Phoenix Inverter 12V 375VA 120V",
        "0xA24A" => "Phoenix Inverter 24V 375VA 120V",
        "0xA24C" => "Phoenix Inverter 48V 375VA 120V",
        "0xA251" => "Phoenix Inverter 12V 500VA 230V",
        "0xA252" => "Phoenix Inverter 24V 500VA 230V",
        "0xA254" => "Phoenix Inverter 48V 500VA 230V",
        "0xA259" => "Phoenix Inverter 12V 500VA 120V",
        "0xA25A" => "Phoenix Inverter 24V 500VA 120V",
        "0xA25C" => "Phoenix Inverter 48V 500VA 120V",
        "0xA261" => "Phoenix Inverter 12V 800VA 230V",
        "0xA262" => "Phoenix Inverter 24V 800VA 230V",
        "0xA264" => "Phoenix Inverter 48V 800VA 230V",
        "0xA269" => "Phoenix Inverter 12V 800VA 120V",
        "0xA26A" => "Phoenix Inverter 24V 800VA 120V",
        "0xA26C" => "Phoenix Inverter 48V 800VA 120V",
        "0xA271" => "Phoenix Inverter 12V 1200VA 230V",
        "0xA272" => "Phoenix Inverter 24V 1200VA 230V",
        "0xA274" => "Phoenix Inverter 48V 1200VA 230V",
        "0xA279" => "Phoenix Inverter 12V 1200VA 120V",
        "0xA27A" => "Phoenix Inverter 24V 1200VA 120V",
        "0xA27C" => "Phoenix Inverter 48V 1200VA 120V",
        "0xA281" => "Phoenix Inverter 12V 1600VA 230V",
        "0xA282" => "Phoenix Inverter 24V 1600VA 230V",
        "0xA284" => "Phoenix Inverter 48V 1600VA 230V",
        "0xA291" => "Phoenix Inverter 12V 2000VA 230V",
        "0xA292" => "Phoenix Inverter 24V 2000VA 230V",
        "0xA294" => "Phoenix Inverter 48V 2000VA 230V",
        "0xA2A1" => "Phoenix Inverter 12V 3000VA 230V",
        "0xA2A2" => "Phoenix Inverter 24V 3000VA 230V",
        "0xA2A4" => "Phoenix Inverter 48V 3000VA 230V",
        "0xA340" => "Phoenix Smart IP43 Charger 12|50 (1+1)",
        "0xA341" => "Phoenix Smart IP43 Charger 12|50 (3)",
        "0xA342" => "Phoenix Smart IP43 Charger 24|25 (1+1)",
        "0xA343" => "Phoenix Smart IP43 Charger 24|25 (3)",
        "0xA344" => "Phoenix Smart IP43 Charger 12|30 (1+1)",
        "0xA345" => "Phoenix Smart IP43 Charger 12|30 (3)",
        "0xA346" => "Phoenix Smart IP43 Charger 24|16 (1+1)",
        "0xA347" => "Phoenix Smart IP43 Charger 24|16 (3)",
        _ => { warn!("Unknown pid: {}", pid); "Unknown" },
    }.to_string()
}

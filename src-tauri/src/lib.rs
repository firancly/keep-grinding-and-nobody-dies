// Notes for morning:
// Loop for port handle (think about adding port into appstate)


use std::{collections::HashMap, ptr::read, sync::Mutex, time::Duration};
use tokio_util::{bytes::buf, sync::CancellationToken};
use tauri::{AppHandle, Manager, State};
use rand::{
    Rng, RngExt, distr::{Distribution, StandardUniform},
};
use serialport::*;
use std::io::{Write, Read};

#[derive(Debug, Clone, Default)]
struct AppState {
    Bomb: HashMap<String, Bomb>,
    TH1CANCELTOKEN: CancellationToken,
}

#[derive(Clone, Debug, Default)]
enum BombType {
    #[default]
    Alpha,
    Beta,
    Omega,
}

#[derive(   Clone, Debug)]
enum EspAction {
    CutRedCable = 0x01,
    CutBlueCable = 0x02,
    CutGreenCable = 0x03,
    CutWhiteCable = 0x04,
    CutBlackCable = 0x05,
    CutOrangeCable = 0x06,
    CutYellowCable = 0x07,
    PressRedButton = 0x08,
    PressBlueButton = 0x09,
    PressGreenButton = 0x0A,
    Nan = 0xFF,
    Error = 0x00,
    Success = 0x0B,
}

#[derive(Clone, Debug, Default)]
enum BombPower {
    #[default]
    Battery,
    Nuclear,
    Electric,
}

#[derive(Clone, Debug, Default)]
enum EventAction {
    #[default]
    UpsideDown,
    Dyslexia,
    FakeBlueScreen,
    TurkAttack,
    FnafJumpscare,
}

#[derive(Clone, Debug, PartialEq)]
enum CableColor {
    Red,
    Blue,
    Green,
    White,
    Black,
    Orange,
    Yellow,
}

#[derive(Clone, Debug, PartialEq)]
enum ButtonColor {
    Red,
    Blue,
    Green,
}

#[derive(Clone, Debug, PartialEq)]
enum BombAction {
    CutCable(CableColor),
    PressButton(ButtonColor),
    Success,
    Error,
    Nan,
}

enum RandType {
    BombType,
    BombPower,
    BombTemperature,
    BombAction,
    Event,
}

#[derive(Clone, Debug, Default)]
struct Event {
    Name: String,
    Description: String,
    Action: EventAction,
}

#[derive(Clone, Default, Debug)]
struct Bomb {
    Action: Vec<BombAction>,
    RandomValues: RandomValues,
    WrongActions: u8,
}

#[derive(Clone, Debug, Default)]
struct RandomValues {
    BombType: BombType,
    BombPower: BombPower,
    BombTemperature: u8,
}


impl Distribution<BombType> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> BombType {
        match rng.random_range(0..3) {
            0 => BombType::Alpha,
            1 => BombType::Beta,
            _ => BombType::Omega,
        }
    }
}

impl Distribution<BombPower> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> BombPower {
        match rng.random_range(0..3) {
            0 => BombPower::Battery,
            1 => BombPower::Nuclear,
            _ => BombPower::Electric,
        }
    }
}

impl Distribution<CableColor> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> CableColor {
        match rng.random_range(0..7) {
            0 => CableColor::Red,
            1 => CableColor::Blue,
            2 => CableColor::Green,
            3 => CableColor::White,
            4 => CableColor::Black,
            5 => CableColor::Orange,
            _ => CableColor::Yellow,
        }
    }
}

impl Distribution<ButtonColor> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> ButtonColor {
        match rng.random_range(0..3) {
            0 => ButtonColor::Red,
            1 => ButtonColor::Blue,
            _ => ButtonColor::Green,
        }
    }
}

impl Distribution<BombAction> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> BombAction {
        if rng.random_bool(0.5) {
            BombAction::CutCable(rng.random())
        } else {
            BombAction::PressButton(rng.random())
        }
    }
}

impl Distribution<EventAction> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> EventAction {
        match rng.random_range(0..5) {
            0 => EventAction::UpsideDown,
            1 => EventAction::Dyslexia,
            2 => EventAction::FakeBlueScreen,
            3 => EventAction::TurkAttack,
            _ => EventAction::FnafJumpscare,
        }
    }
}

impl Distribution<Event> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Event {
        let action: EventAction = rng.random();
        let (name, desc) = match action {
            EventAction::UpsideDown => ("Upside Down", "Screen is upside down!"),
            EventAction::Dyslexia => ("Dyslexia", "Texts are mixed up!"),
            EventAction::FakeBlueScreen => ("Fake Blue Screen", "System looks crashed!"),
            EventAction::TurkAttack => ("Turk Attack", "There is a cyber attack from turks on the system!"),
            EventAction::FnafJumpscare => ("Fnaf Jumpscare", "A scary image appeared!"),
        };

        Event {
            Name: name.to_string(),
            Description: desc.to_string(),
            Action: action,
        }
    }
}


#[derive(Debug, Clone)]
enum RandResult {
    BombType(BombType),
    BombPower(BombPower),
    BombTemperature(u8),
    BombAction(BombAction),
    Event(Event),
}

async fn random(random_type: RandType) -> RandResult {
    let mut rng = rand::rng();

    let result = match random_type {
        RandType::BombType => RandResult::BombType(rng.random()),
        RandType::BombPower => RandResult::BombPower(rng.random()),
        RandType::BombTemperature => RandResult::BombTemperature(rng.random_range(0..=100)),
        RandType::Event => RandResult::Event(rng.random()),
        RandType::BombAction => RandResult::BombAction(rng.random()),
    };

    result
}

impl Bomb {
    async fn generate() -> Self {
        let mut rng = rand::rng();

        let action_count = rng.random_range(3..=6);
        let actions: Vec<BombAction> = (0..action_count).map(|_| rng.random()).collect();

        Bomb {
            Action: actions,
            RandomValues: RandomValues {
                BombType: rng.random(),
                BombPower: rng.random(),
                BombTemperature: rng.random_range(0..=100),
            },
            WrongActions: 0,
        }
    }


    async fn start(&mut self, handle: AppHandle) {
        let cancellation_token = CancellationToken::new();

        let worker_token = cancellation_token.clone();
        let worker_handle = handle.clone();
        let bomb_clone = self.clone();

        tokio::spawn(async move {
            handle_port(bomb_clone, worker_token, worker_handle).await;
        });

        let state = handle.state::<Mutex<AppState>>();
        let mut app_state = state.lock().unwrap();
        app_state.Bomb.insert(format!("APP"), self.clone());
        app_state.TH1CANCELTOKEN = cancellation_token;

    }

}

async fn handle_port(Bomb: Bomb, cancellationtoken : CancellationToken, handle : AppHandle) -> std::result::Result<(), String> {
    let mut conn_port = serialport::new("/dev/ttyUSB0", 115200)
    .timeout(Duration::from_millis(50))
    .open()
    .expect("Failed to open port");

    let mut buffer: Vec<u8> = vec![0; 64];

    let worker_token = cancellationtoken.clone();
    let mut worker_handle = handle.clone();
    let mut worker_bomb = Bomb.clone();

    match conn_port.read(buffer.as_mut_slice()) {
        Ok(bytes_read) if bytes_read > 0 => {
            let esp_action = parse_esp_byte(buffer[0]).unwrap_or_else(|e| {
                eprintln!("Error parsing ESP action: {}", e);
                return EspAction::Nan;
            });
            tokio::spawn( async move {
                let action_handler = handle_bomb_action(&mut worker_bomb, parse_esp_action(esp_action), worker_handle).await;
                if action_handler {
                    println!("Action handled successfully.");
                    conn_port.write_all(&[0x0B]).expect("Failed to write to port");
                } else {
                    println!("Wrong action attempted. Total wrong actions: {}", worker_bomb.WrongActions);
                    conn_port.write_all(&[0xFF]).expect("Failed to write to port");
                }
            });

            Ok(())
        }
        Ok(_) => Err("Veri gelmedi (0 byte)".into()),
        Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
            Ok(())
        }
        Err(e) => Err(format!("Port read error: {}", e)),
    }

}

async fn handle_bomb_action(bomb: &mut Bomb, action: BombAction, handle: AppHandle) -> bool{
    if bomb.Action.contains(&action) {
        bomb.Action.retain(|a| a != &action);
        true
    } else {
        bomb.WrongActions += 1;
        false
    }
}

async fn handle_event(event: Event, handle: AppHandle) {
    
}

#[tauri::command]
async fn start_game() -> std::result::Result<(), String> {
    



    Ok(())
}

fn parse_esp_byte(byte: u8) -> std::result::Result<EspAction, String> {
    match byte {
        0x01 => Ok(EspAction::CutRedCable),
        0x02 => Ok(EspAction::CutBlueCable),
        0x03 => Ok(EspAction::CutGreenCable),
        0x04 => Ok(EspAction::CutWhiteCable),
        0x05 => Ok(EspAction::CutBlackCable),
        0x06 => Ok(EspAction::CutOrangeCable),
        0x07 => Ok(EspAction::CutYellowCable),
        0x08 => Ok(EspAction::PressRedButton),
        0x09 => Ok(EspAction::PressBlueButton),
        0x0A => Ok(EspAction::PressGreenButton),
        _ => Err(format!("Unknown ESP action byte: {}", byte)),
    }
}

fn parse_esp_action(action: EspAction) -> BombAction {
    match action {
        EspAction::CutRedCable => BombAction::CutCable(CableColor::Red),
        EspAction::CutBlueCable => BombAction::CutCable(CableColor::Blue),
        EspAction::CutGreenCable => BombAction::CutCable(CableColor::Green),
        EspAction::CutWhiteCable => BombAction::CutCable(CableColor::White),
        EspAction::CutBlackCable => BombAction::CutCable(CableColor::Black),
        EspAction::CutOrangeCable => BombAction::CutCable(CableColor::Orange),
        EspAction::CutYellowCable => BombAction::CutCable(CableColor::Yellow),
        EspAction::PressRedButton => BombAction::PressButton(ButtonColor::Red),
        EspAction::PressBlueButton => BombAction::PressButton(ButtonColor::Blue),
        EspAction::PressGreenButton => BombAction::PressButton(ButtonColor::Green),
        EspAction::Nan => {
            eprintln!("Received Nan action from ESP");
            BombAction::Nan
        },
        EspAction::Error => {
            eprintln!("Received Error action from ESP");
            BombAction::Error
        },
        EspAction::Success => {
            eprintln!("Received Success action from ESP");
            BombAction::Success
        },
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app|{
            let app_handle = app.handle();
            app.manage(Mutex::new(AppState::default()));
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![

        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

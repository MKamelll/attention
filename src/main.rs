use core::time;
use std::io::{Stderr, Stdout};
use std::os::linux::raw::stat;
use std::os::unix::process;
use std::process::{ChildStdin, Command, Stdio};
use std::{env, string};
use std::thread::sleep;

#[derive(PartialEq, Eq)]
enum ScreenBlankingState {
    Off,
    On
}

#[derive(PartialEq, Eq)]
enum FullscreenState {
    NotFullscreen,
    Fullscreen
}

#[derive(PartialEq, Eq)]
enum TrackAudioState {
    On,
    Off
}

struct State {
    last_screen_blanking_state: ScreenBlankingState,
    last_fullscreen_state: FullscreenState,
    last_track_audio_state: TrackAudioState
}

impl State {
    fn new() -> Self {
        Self {
            last_screen_blanking_state: ScreenBlankingState::On,
            last_fullscreen_state: FullscreenState::NotFullscreen,
            last_track_audio_state: TrackAudioState::Off
        }
    }
}

fn help() -> &'static str {
    let help = "
        attention <flag> <app_name>
        Flags:
            --track-audio       Track audio to disable power management
            --track-fullscreen  Track fullscreen to diable power management
    ";
    help
}

fn launch_app(app_name: &String, args: &String) -> u32 {
    let process =
    Command::new(app_name)
    .arg(args)
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .spawn()
    .expect(&format!("Couldn't launch {}", app_name));

    process.id()
}

fn wait_for_window_to_show_up(app_name: &String, pid: u32) -> String {
    loop {
        let output =
        Command::new("wmctrl")
        .arg("-lp")
        .output()
        .expect("Failed to run wmctrl trying to wait for the window.");

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();
            for line in stdout.lines() {
                if line.contains(&pid.to_string()) && line.contains(app_name) {
                    if let Some(window_id) = line.split_whitespace().next() {
                        return window_id.to_owned();
                    }
                }    
            }
            
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            panic!("Command wmctrl returned error trying to check if window appeared: {}", stderr);
        }

        sleep(time::Duration::from_millis(200));
    }
}

fn is_window_closed(app_name: &String, pid: u32, state: &mut State) -> bool {
    let output =
    Command::new("wmctrl")
    .arg("-lp")
    .output()
    .expect("Failed to run wmctrl trying to check if the window is closed.");

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();
        if stdout.contains(&pid.to_string()) && stdout.contains(app_name) {
            return false;
        }
        println!("{}'s window is closed..", app_name);
        turn_on_screen_blanking(state);
        println!("Shutting down..");    
        return true;
    }
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    panic!("Command wmctrl returned error trying to check if the window closed: {}", stderr);    
}

fn is_window_fullscreen(window_id: &String) -> bool {
    let output =
    Command::new("xprop")
    .arg("-id")
    .arg(window_id)
    .output()
    .expect("Failed to run xprop trying to check if window is fullscreen.");

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();
        let property = "_NET_WM_STATE(ATOM) = _NET_WM_STATE_FULLSCREEN".to_owned().to_lowercase();
        if stdout.contains(&property) {
            return true;
        }
        return false;
    }
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    panic!("Command xprop returned error trying to check if the window is fullscreen: {}", stderr);
}

fn is_playing_audio(app_name: &String) -> bool {
    let output =
    Command::new("pactl")
    .arg("list")
    .arg("sink-inputs")
    .output()
    .expect("Failed to run pactl trying to check if the app is playing audio.");

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();
        let stream_is_live = "stream.is-live = \"true\"";
        let stream_not_paused = "corked: no";
        if stdout.contains(app_name) && stdout.contains(stream_is_live) && stdout.contains(stream_not_paused) {
            return true;
        }
        return false;
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    panic!("Command pactl returned error trying to check if the window is playing audio: {}", stderr);
}

fn turn_off_screen_blanking(app_name: &String, state: &mut State) {
    if state.last_screen_blanking_state == ScreenBlankingState::On {
        println!("Turning off screen blanking..");

        let notify_output =
        Command::new("notify-send")
        .arg(format!("⚠️ Power Management is inhibited by {}", app_name))
        .output()
        .expect("Failed to run notify-send trying to check if we can send a notification with disabling power management.");

        if !notify_output.status.success() {
            let stderr = String::from_utf8_lossy(&notify_output.stderr);
            panic!("notify-send returned error when trying to send a power management disabling notification: {}", stderr);
        }

        let power_down_output =
        Command::new("xset")
        .arg("-dpms")
        .output()
        .expect("Failed to run xset to power down");
        
        if !power_down_output.status.success() {
            let stderr = String::from_utf8_lossy(&power_down_output.stderr);
            panic!("xset returned error when trying to power down: {}", stderr);
        }

        state.last_screen_blanking_state = ScreenBlankingState::Off;
    }
}

fn turn_on_screen_blanking(state: &mut State) {
    if state.last_screen_blanking_state == ScreenBlankingState::Off {
        println!("Turning on screen blanking..");

        let notify_output =
        Command::new("notify-send")
        .arg("⚠️ Power Management is back to normal")
        .output()
        .expect("Failed to run notify-send trying to check if we can send a notification with enabling power management.");

        if !notify_output.status.success() {
            let stderr = String::from_utf8_lossy(&notify_output.stderr);
            panic!("notify-send returned error when trying to send a power management enabling notification: {}", stderr);
        }

        let power_down_output =
        Command::new("xset")
        .arg("+dpms")
        .output()
        .expect("Failed to run xset to power up");
        
        if !power_down_output.status.success() {
            let stderr = String::from_utf8_lossy(&power_down_output.stderr);
            panic!("xset returned error when trying to power up: {}", stderr);
        }

        state.last_screen_blanking_state = ScreenBlankingState::On;
    }
}

fn we_are_tracking_fullscreen(app_name: &String, window_id: &String, state: &mut State) {
    if is_window_fullscreen(window_id) {
        if state.last_fullscreen_state == FullscreenState::NotFullscreen {
            println!("{} is now fullscreen..", app_name);
            state.last_fullscreen_state = FullscreenState::Fullscreen;
            turn_off_screen_blanking(app_name, state);
        }
    } else {
        if state.last_fullscreen_state == FullscreenState::Fullscreen {
            println!("{} is no longer fullscreen..", app_name);
            state.last_fullscreen_state = FullscreenState::NotFullscreen;
            turn_on_screen_blanking(state);
        }
    }
}

fn we_are_tracking_audio(app_name: &String, state: &mut State) {
    if is_playing_audio(app_name) {
        if state.last_track_audio_state == TrackAudioState::Off {
            println!("{} is now playing audio..", app_name);
            state.last_track_audio_state = TrackAudioState::On;
            turn_off_screen_blanking(app_name, state);
        }
    } else {
        if state.last_track_audio_state == TrackAudioState::On {
            println!("{} is no longer playing audio..", app_name);
            state.last_track_audio_state = TrackAudioState::Off;
            turn_on_screen_blanking(state);
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut track_audio = false;
    let mut track_fullscreen = false;
    
    if args.len() < 3 {
        println!("{}", help());
        panic!("Not enough arguments.")
    }
    
    let track_flag = &args[1];
    let app_name = &args[2];
    let mut app_args: Option<String> = None;

    if args.len() > 3 {
        let _args = &args[3..];
        app_args = Some(_args.join(" "));
    }

    match track_flag.as_str() {
        "--track-audio" => track_audio = !track_audio,
        "--track-fullscreen" => track_fullscreen = !track_fullscreen,
        _ => panic!("Unknown flag {}", track_flag)
    }

    let mut state = State::new();
    let mut pid: u32;

    if let Some(app_args) = app_args {
        pid = launch_app(app_name, &app_args);
    } else {
        pid = launch_app(app_name, &String::new());
    }

    let window_id = wait_for_window_to_show_up(app_name, pid);

    while !is_window_closed(app_name, pid, &mut state) {
        if track_audio {
            we_are_tracking_audio(app_name, &mut state);
        } else if track_fullscreen {
            we_are_tracking_fullscreen(app_name, &window_id, &mut state);
        }

        sleep(time::Duration::from_secs(1));
    }
}

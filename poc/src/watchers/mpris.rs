use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use zbus::blocking::Connection;
use zbus::blocking::MessageIterator;
use zbus::zvariant::{OwnedValue, Value};
use zbus::MatchRule;
use zbus::MessageType;

use crate::state::FullState;

const MPRIS_PREFIX: &str = "org.mpris.MediaPlayer2.";
const PLAYER_IFACE: &str = "org.mpris.MediaPlayer2.Player";
const PLAYER_PATH: &str = "/org/mpris/MediaPlayer2";
const PROPS_IFACE: &str = "org.freedesktop.DBus.Properties";
const DBUS_IFACE: &str = "org.freedesktop.DBus";

fn short_name(bus_name: &str) -> String {
    bus_name
        .strip_prefix(MPRIS_PREFIX)
        .unwrap_or(bus_name)
        .to_string()
}

fn list_players(conn: &Connection) -> Vec<String> {
    let reply = match conn.call_method(
        Some(DBUS_IFACE),
        "/org/freedesktop/DBus",
        Some(DBUS_IFACE),
        "ListNames",
        &(),
    ) {
        Ok(r) => r,
        Err(e) => {
            log::warn!("[mpris] ListNames failed: {e}");
            return vec![];
        }
    };

    let names: Vec<String> = match reply.body() {
        Ok(n) => n,
        Err(e) => {
            log::warn!("[mpris] failed to deserialize ListNames: {e}");
            return vec![];
        }
    };

    names
        .into_iter()
        .filter(|n| n.starts_with(MPRIS_PREFIX))
        .collect()
}

fn get_string_prop(conn: &Connection, dest: &str, prop: &str) -> Option<String> {
    let reply = conn
        .call_method(
            Some(dest),
            PLAYER_PATH,
            Some(PROPS_IFACE),
            "Get",
            &(PLAYER_IFACE, prop),
        )
        .ok()?;

    let val: OwnedValue = reply.body().ok()?;
    let v: Value = (&val).into();
    unwrap_string(v)
}

fn unwrap_string(v: Value<'_>) -> Option<String> {
    match v {
        Value::Str(s) => Some(s.to_string()),
        Value::Value(inner) => unwrap_string(*inner),
        _ => None,
    }
}

fn get_metadata(conn: &Connection, dest: &str) -> HashMap<String, OwnedValue> {
    let reply = match conn.call_method(
        Some(dest),
        PLAYER_PATH,
        Some(PROPS_IFACE),
        "Get",
        &(PLAYER_IFACE, "Metadata"),
    ) {
        Ok(r) => r,
        Err(_) => return HashMap::new(),
    };

    let val: OwnedValue = match reply.body() {
        Ok(v) => v,
        Err(_) => return HashMap::new(),
    };

    let v: Value = (&val).into();
    extract_metadata_map(v)
}

fn extract_metadata_map(v: Value<'_>) -> HashMap<String, OwnedValue> {
    match v {
        Value::Dict(d) => {
            <HashMap<String, OwnedValue>>::try_from(d).unwrap_or_default()
        }
        Value::Value(inner) => extract_metadata_map(*inner),
        _ => HashMap::new(),
    }
}

fn meta_string(meta: &HashMap<String, OwnedValue>, key: &str) -> String {
    let Some(val) = meta.get(key) else {
        return String::new();
    };
    let v: Value = val.into();
    unwrap_string(v).unwrap_or_default()
}

fn meta_artist(meta: &HashMap<String, OwnedValue>) -> String {
    let Some(val) = meta.get("xesam:artist") else {
        return String::new();
    };
    let v: Value = val.into();
    extract_artist_list(v)
}

fn extract_artist_list(v: Value<'_>) -> String {
    match v {
        Value::Array(arr) => {
            let artists: Vec<String> = arr
                .iter()
                .filter_map(|item| match item {
                    Value::Str(s) => Some(s.to_string()),
                    _ => None,
                })
                .collect();
            artists.join(", ")
        }
        Value::Value(inner) => extract_artist_list(*inner),
        _ => String::new(),
    }
}

fn get_playback_status(conn: &Connection, dest: &str) -> Option<String> {
    get_string_prop(conn, dest, "PlaybackStatus")
}

fn pick_active_player(conn: &Connection, players: &[String]) -> Option<String> {
    if players.is_empty() {
        return None;
    }
    for p in players {
        if get_playback_status(conn, p).as_deref() == Some("Playing") {
            return Some(p.clone());
        }
    }
    for p in players {
        if get_playback_status(conn, p).as_deref() == Some("Paused") {
            return Some(p.clone());
        }
    }
    Some(players[0].clone())
}

fn read_player_state(
    conn: &Connection,
    player: &str,
    state: &Arc<Mutex<FullState>>,
    dirty: &Arc<AtomicBool>,
) {
    let status = get_playback_status(conn, player).unwrap_or_default();
    let playing = status == "Playing";
    let meta = get_metadata(conn, player);

    let title = meta_string(&meta, "xesam:title");
    let artist = meta_artist(&meta);
    let album = meta_string(&meta, "xesam:album");
    let art_url = meta_string(&meta, "mpris:artUrl");
    let name = short_name(player);

    if let Ok(mut s) = state.lock() {
        s.media_playing = playing;
        s.media_title = title;
        s.media_artist = artist;
        s.media_album = album;
        s.media_art_url = art_url;
        s.media_player = name;
    }
    dirty.store(true, Ordering::Relaxed);
}

fn clear_media_state(state: &Arc<Mutex<FullState>>, dirty: &Arc<AtomicBool>) {
    if let Ok(mut s) = state.lock() {
        s.media_playing = false;
        s.media_title.clear();
        s.media_artist.clear();
        s.media_album.clear();
        s.media_art_url.clear();
        s.media_player.clear();
    }
    dirty.store(true, Ordering::Relaxed);
}

fn find_player_for_sender(conn: &Connection, sender: &str) -> Option<String> {
    let players = list_players(conn);
    for p in &players {
        let reply = conn
            .call_method(
                Some(DBUS_IFACE),
                "/org/freedesktop/DBus",
                Some(DBUS_IFACE),
                "GetNameOwner",
                &(p.as_str(),),
            )
            .ok()?;
        let owner: String = reply.body().ok()?;
        if owner == sender {
            return Some(p.clone());
        }
    }
    None
}

pub fn start_mpris_watcher(state: Arc<Mutex<FullState>>, dirty: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let conn = match Connection::session() {
            Ok(c) => c,
            Err(e) => {
                log::warn!("[mpris] failed to connect to session bus: {e}");
                return;
            }
        };

        let mut players = list_players(&conn);
        let mut active = pick_active_player(&conn, &players);

        if let Some(ref player) = active {
            read_player_state(&conn, player, &state, &dirty);
        } else {
            clear_media_state(&state, &dirty);
        }

        // Use a broad match rule: all signals on the session bus.
        // We filter by interface/member in code.
        let rule = MatchRule::builder()
            .msg_type(MessageType::Signal)
            .build();

        let iter = match MessageIterator::for_match_rule(rule, &conn, Some(64)) {
            Ok(it) => it,
            Err(e) => {
                log::warn!("[mpris] failed to create signal iterator: {e}");
                return;
            }
        };

        for msg_result in iter {
            let msg = match msg_result {
                Ok(m) => m,
                Err(e) => {
                    log::warn!("[mpris] signal iterator error: {e}");
                    break;
                }
            };

            let Ok(header) = msg.header() else { continue };
            if header.message_type() != Ok(MessageType::Signal) {
                continue;
            }

            let iface = header
                .interface()
                .ok()
                .flatten()
                .map(|i| i.as_str().to_string())
                .unwrap_or_default();
            let member = header
                .member()
                .ok()
                .flatten()
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();

            // NameOwnerChanged: player appeared or disappeared
            if iface == DBUS_IFACE && member == "NameOwnerChanged" {
                let Ok((name, _old_owner, new_owner)) =
                    msg.body::<(String, String, String)>()
                else {
                    continue;
                };

                if !name.starts_with(MPRIS_PREFIX) {
                    continue;
                }

                if new_owner.is_empty() {
                    log::info!("[mpris] player removed: {}", short_name(&name));
                    players.retain(|p| p != &name);
                    if active.as_deref() == Some(&name) {
                        active = pick_active_player(&conn, &players);
                        if let Some(ref player) = active {
                            read_player_state(&conn, player, &state, &dirty);
                        } else {
                            clear_media_state(&state, &dirty);
                        }
                    }
                } else {
                    log::info!("[mpris] player appeared: {}", short_name(&name));
                    if !players.contains(&name) {
                        players.push(name.clone());
                    }
                    active = pick_active_player(&conn, &players);
                    if let Some(ref player) = active {
                        read_player_state(&conn, player, &state, &dirty);
                    }
                }
                continue;
            }

            // PropertiesChanged: playback or metadata update
            if iface == PROPS_IFACE && member == "PropertiesChanged" {
                let Ok((changed_iface, _changed_props, _invalidated)) =
                    msg.body::<(String, HashMap<String, OwnedValue>, Vec<String>)>()
                else {
                    continue;
                };

                if changed_iface != PLAYER_IFACE {
                    continue;
                }

                let sender = header
                    .sender()
                    .ok()
                    .flatten()
                    .map(|s| s.as_str().to_string())
                    .unwrap_or_default();

                let signal_player = find_player_for_sender(&conn, &sender);

                if let Some(ref sp) = signal_player {
                    let status = get_playback_status(&conn, sp).unwrap_or_default();
                    if status == "Playing" && active.as_ref() != Some(sp) {
                        active = Some(sp.clone());
                    }
                    if active.as_ref() == Some(sp) {
                        read_player_state(&conn, sp, &state, &dirty);
                    }
                } else if let Some(ref player) = active {
                    read_player_state(&conn, player, &state, &dirty);
                }

                // If active player stopped, find a better one
                if let Some(ref player) = active {
                    if get_playback_status(&conn, player).as_deref() == Some("Stopped") {
                        let new_active = pick_active_player(&conn, &players);
                        if new_active != active {
                            active = new_active;
                            if let Some(ref p) = active {
                                read_player_state(&conn, p, &state, &dirty);
                            } else {
                                clear_media_state(&state, &dirty);
                            }
                        }
                    }
                }
            }
        }
    });
}

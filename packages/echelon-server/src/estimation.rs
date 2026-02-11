use std::io::Cursor;

/// Time multiplier used to more accurately estimate ETAs.
/// Set the TIME_MULTIPLIER environment variable to override this.
const DEFAULT_TIME_MULTIPLIER: f64 = 1.0;

// yrpX file constants
const REPLAY_YRPX: u32 = 0x58707279;
const REPLAY_YRP1: u32 = 0x31707279;
const REPLAY_COMPRESSED: u32 = 0x1;
const REPLAY_EXTENDED_HEADER: u32 = 0x200;
const REPLAY_SINGLE_MODE: u32 = 0x8;
const REPLAY_NEWREPLAY: u32 = 0x20;
const REPLAY_TAG: u32 = 0x2;
const REPLAY_64BIT_DUELFLAG: u32 = 0x100;

// Message types (from ocgapi_constants.h)
const MSG_HINT: u8 = 2;
const MSG_START: u8 = 4;
const MSG_WIN: u8 = 5;
const MSG_UPDATE_DATA: u8 = 6;
const MSG_UPDATE_CARD: u8 = 7;
const MSG_CONFIRM_DECKTOP: u8 = 30;
const MSG_CONFIRM_CARDS: u8 = 31;
const MSG_SHUFFLE_DECK: u8 = 32;
const MSG_SHUFFLE_HAND: u8 = 33;
const MSG_SHUFFLE_SET_CARD: u8 = 36;
const MSG_NEW_TURN: u8 = 40;
const MSG_NEW_PHASE: u8 = 41;
const MSG_CONFIRM_EXTRATOP: u8 = 42;
const MSG_MOVE: u8 = 50;
const MSG_POS_CHANGE: u8 = 53;
const MSG_SET: u8 = 54;
const MSG_SWAP: u8 = 55;
const MSG_SUMMONING: u8 = 60;
const MSG_SUMMONED: u8 = 61;
const MSG_SPSUMMONING: u8 = 62;
const MSG_SPSUMMONED: u8 = 63;
const MSG_FLIPSUMMONING: u8 = 64;
const MSG_FLIPSUMMONED: u8 = 65;
const MSG_CHAINING: u8 = 70;
const MSG_CHAINED: u8 = 71;
const MSG_CHAIN_SOLVING: u8 = 72;
const MSG_CHAIN_SOLVED: u8 = 73;
const MSG_CHAIN_END: u8 = 74;
const MSG_CHAIN_NEGATED: u8 = 75;
const MSG_CHAIN_DISABLED: u8 = 76;
const MSG_RANDOM_SELECTED: u8 = 81;
const MSG_BECOME_TARGET: u8 = 83;
const MSG_DRAW: u8 = 90;
const MSG_DAMAGE: u8 = 91;
const MSG_RECOVER: u8 = 92;
const MSG_EQUIP: u8 = 93;
const MSG_LPUPDATE: u8 = 94;
const MSG_UNEQUIP: u8 = 95;
const MSG_CARD_TARGET: u8 = 96;
const MSG_CANCEL_TARGET: u8 = 97;
const MSG_PAY_LPCOST: u8 = 100;
const MSG_ADD_COUNTER: u8 = 101;
const MSG_REMOVE_COUNTER: u8 = 102;
const MSG_ATTACK: u8 = 110;
const MSG_BATTLE: u8 = 111;
const MSG_ATTACK_DISABLED: u8 = 112;
const MSG_TOSS_COIN: u8 = 130;
const MSG_TOSS_DICE: u8 = 131;
const MSG_TAG_SWAP: u8 = 161;
const MSG_RELOAD_FIELD: u8 = 162;

// Hint types
const HINT_OPSELECTED: u8 = 4;
const HINT_EFFECT: u8 = 5;
const HINT_RACE: u8 = 6;
const HINT_ATTRIB: u8 = 7;
const HINT_CODE: u8 = 8;
const HINT_NUMBER: u8 = 9;
const HINT_CARD: u8 = 10;
const HINT_ZONE: u8 = 11;
const HINT_SKILL: u8 = 200;
const HINT_SKILL_COVER: u8 = 201;
const HINT_SKILL_FLIP: u8 = 202;
const HINT_SKILL_REMOVE: u8 = 203;

#[derive(Debug, Clone)]
pub struct Packet {
    pub message: u8,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct ReplayHeader {
    pub flag: u32,
    pub datasize: u32,
    pub props: [u8; 5],
    pub header_size: usize,
}

#[derive(Debug)]
pub enum ReplayError {
    IoError(std::io::Error),
    FileTooSmall,
    InvalidReplayId(u32),
    DecompressionError(String),
}

impl From<std::io::Error> for ReplayError {
    fn from(e: std::io::Error) -> Self {
        ReplayError::IoError(e)
    }
}

impl std::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReplayError::IoError(e) => write!(f, "IO error: {}", e),
            ReplayError::FileTooSmall => write!(f, "File too small to be a valid replay"),
            ReplayError::InvalidReplayId(id) => write!(f, "Invalid replay ID: 0x{:08x}", id),
            ReplayError::DecompressionError(s) => write!(f, "Decompression error: {}", s),
        }
    }
}

impl std::error::Error for ReplayError {}

fn read_u32_le(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

fn read_header(data: &[u8]) -> Result<ReplayHeader, ReplayError> {
    if data.len() < 32 {
        return Err(ReplayError::FileTooSmall);
    }

    let id = read_u32_le(data, 0);
    let flag = read_u32_le(data, 8);
    let datasize = read_u32_le(data, 16);

    let mut props = [0u8; 5];
    props.copy_from_slice(&data[24..29]);

    if id != REPLAY_YRPX && id != REPLAY_YRP1 {
        return Err(ReplayError::InvalidReplayId(id));
    }

    let header_size = if flag & REPLAY_EXTENDED_HEADER != 0 {
        72
    } else {
        32
    };
    if data.len() < header_size {
        return Err(ReplayError::FileTooSmall);
    }

    Ok(ReplayHeader {
        flag,
        datasize,
        props,
        header_size,
    })
}

fn decompress_lzma(
    data: &[u8],
    props: &[u8; 5],
    uncompressed_size: u32,
) -> Result<Vec<u8>, ReplayError> {
    // Build LZMA alone format: props(5) + uncompressed_size(8 LE) + compressed_data
    let mut lzma_stream = Vec::with_capacity(13 + data.len());
    lzma_stream.extend_from_slice(props);
    lzma_stream.extend_from_slice(&(uncompressed_size as u64).to_le_bytes());
    lzma_stream.extend_from_slice(data);

    let mut decompressed = Vec::new();
    let mut reader = Cursor::new(&lzma_stream);

    lzma_rs::lzma_decompress(&mut reader, &mut decompressed)
        .map_err(|e| ReplayError::DecompressionError(e.to_string()))?;

    Ok(decompressed)
}

fn parse_packets(data: &[u8], flag: u32) -> Vec<Packet> {
    let mut packets = Vec::new();
    let mut offset = 0;

    // Skip names section
    if flag & REPLAY_SINGLE_MODE != 0 {
        offset += 80; // 2 names × 40 bytes
    } else {
        let home_count = if flag & REPLAY_NEWREPLAY != 0 {
            let c = read_u32_le(data, offset) as usize;
            offset += 4;
            c
        } else if flag & REPLAY_TAG != 0 {
            2
        } else {
            1
        };
        offset += home_count * 40;

        let oppo_count = if flag & REPLAY_NEWREPLAY != 0 {
            let c = read_u32_le(data, offset) as usize;
            offset += 4;
            c
        } else if flag & REPLAY_TAG != 0 {
            2
        } else {
            1
        };
        offset += oppo_count * 40;
    }

    // Skip duel_flags
    offset += if flag & REPLAY_64BIT_DUELFLAG != 0 {
        8
    } else {
        4
    };

    // Parse packets: message(1) + length(4) + data(length)
    while offset + 5 <= data.len() {
        let msg = data[offset];
        offset += 1;

        if msg == 0 {
            break;
        }

        let pkt_len = read_u32_le(data, offset) as usize;
        offset += 4;

        if pkt_len > 100000 || offset + pkt_len > data.len() {
            break;
        }

        packets.push(Packet {
            message: msg,
            data: data[offset..offset + pkt_len].to_vec(),
        });
        offset += pkt_len;
    }

    packets
}

/// Load and parse a yrpX replay from a byte slice.
pub fn load_replay_packets(data: &[u8]) -> Result<Vec<Packet>, ReplayError> {
    let header = read_header(data)?;
    let compressed_data = &data[header.header_size..];

    let decompressed = if header.flag & REPLAY_COMPRESSED != 0 {
        decompress_lzma(compressed_data, &header.props, header.datasize)?
    } else {
        compressed_data.to_vec()
    };

    let packets = parse_packets(&decompressed, header.flag);
    Ok(packets)
}

/// Estimate replay duration in seconds.
///
/// # Arguments
/// * `packets` - The parsed packet list from a replay
///
/// # Returns
/// Estimated duration in seconds as f64
pub fn estimate_duration(packets: &[Packet]) -> f64 {
    let mut total_frames: f64 = 0.0;

    for pkt in packets {
        let msg = pkt.message;

        // Skip non-pauseable messages (processed instantly)
        if matches!(
            msg,
            MSG_START
                | MSG_UPDATE_DATA
                | MSG_UPDATE_CARD
                | MSG_SET
                | MSG_SWAP
                | MSG_SUMMONING
                | MSG_SPSUMMONING
                | MSG_FLIPSUMMONING
                | MSG_CHAIN_SOLVING
                | MSG_CHAIN_SOLVED
                | MSG_CHAIN_END
                | MSG_RANDOM_SELECTED
                | MSG_EQUIP
                | MSG_UNEQUIP
                | MSG_CARD_TARGET
                | MSG_CANCEL_TARGET
                | MSG_BATTLE
                | MSG_ATTACK_DISABLED
                | MSG_TAG_SWAP
                | MSG_RELOAD_FIELD
        ) {
            continue;
        }

        total_frames += match msg {
            MSG_WIN => 120.0,
            MSG_NEW_TURN => 30.0,
            MSG_NEW_PHASE => 15.0,
            MSG_HINT => {
                if let Some(&hint) = pkt.data.first() {
                    match hint {
                        HINT_EFFECT | HINT_CARD => 30.0,
                        HINT_OPSELECTED | HINT_RACE | HINT_ATTRIB | HINT_CODE | HINT_NUMBER
                        | HINT_ZONE => 40.0,
                        HINT_SKILL | HINT_SKILL_COVER | HINT_SKILL_FLIP => 11.0,
                        HINT_SKILL_REMOVE => 20.0,
                        _ => 0.0,
                    }
                } else {
                    0.0
                }
            }
            MSG_CHAINING => 40.0,
            MSG_CHAINED => 20.0,
            MSG_CHAIN_NEGATED | MSG_CHAIN_DISABLED => 20.0,
            MSG_SUMMONED | MSG_SPSUMMONED | MSG_FLIPSUMMONED => 30.0,
            MSG_MOVE => 20.0,
            MSG_POS_CHANGE => 20.0,
            MSG_DRAW => {
                let count = pkt.data.get(1).copied().unwrap_or(1) as f64;
                20.0 + count * 10.0
            }
            MSG_DAMAGE | MSG_RECOVER => 60.0,
            MSG_PAY_LPCOST => 30.0,
            MSG_LPUPDATE => 20.0,
            MSG_ATTACK => 40.0,
            MSG_BECOME_TARGET => 20.0,
            MSG_SHUFFLE_DECK => 50.0,
            MSG_SHUFFLE_HAND | MSG_SHUFFLE_SET_CARD => 40.0,
            MSG_CONFIRM_DECKTOP | MSG_CONFIRM_EXTRATOP => {
                let count = pkt.data.get(1).copied().unwrap_or(1).max(1) as f64;
                50.0 * count
            }
            MSG_CONFIRM_CARDS => 40.0,
            MSG_TOSS_COIN | MSG_TOSS_DICE => 30.0,
            MSG_ADD_COUNTER | MSG_REMOVE_COUNTER => 15.0,
            _ => 5.0,
        };
    }

    let time_multiplier = std::env::var("TIME_MULTIPLIER")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(DEFAULT_TIME_MULTIPLIER);

    // Convert frames to seconds (at 60 FPS)
    (total_frames * (1000.0 / 60.0) / 1000.0) * time_multiplier
}

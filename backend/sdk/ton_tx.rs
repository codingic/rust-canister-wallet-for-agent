use num_bigint::BigUint;
use sha2::{Digest, Sha256};

use crate::error::{WalletError, WalletResult};

const BASE64_URL_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
const BASE64_STD_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub const TON_WORKCHAIN_BASECHAIN: i8 = 0;
pub const TON_WALLET_V4R2_WALLET_ID: u32 = 698_983_191; // 0x29A9_A317
pub const TON_JETTON_OP_TRANSFER: u32 = 0x0f8a_7ea5;

// Wallet V4R2 code BOC hex (compiled wallet v4r2 code cell).
// Source mirrored from public TON examples / tonweb wallet sources.
const TON_WALLET_V4R2_CODE_BOC_HEX: &str = "b5ee9c7241021401000281000114ff00f4a413f4bcf2c80b01020120020d020148030402dcd020d749c120915b8f6320d70b1f2082106578746ebd21821073696e74bdb0925f03e082106578746eba8eb48020d72101d074d721fa4030fa44f828fa443058bd915be0ed44d0810141d721f4058307f40e6fa1319130e18040d721707fdb3ce03120d749810280b99130e070e2100f020120050c020120060902016e07080019adce76a2684020eb90eb85ffc00019af1df6a2684010eb90eb858fc00201480a0b0017b325fb51341c75c875c2c7e00011b262fb513435c280200019be5f0f6a2684080a0eb90fa02c0102f20e011e20d70b1f82107369676ebaf2e08a7f0f01e68ef0eda2edfb218308d722028308d723208020d721d31fd31fd31fed44d0d200d31f20d31fd3ffd70a000af90140ccf9109a28945f0adb31e1f2c087df02b35007b0f2d0845125baf2e0855036baf2e086f823bbf2d0882292f800de01a47fc8ca00cb1f01cf16c9ed542092f80fde70db3cd81003f6eda2edfb02f404216e926c218e4c0221d73930709421c700b38e2d01d72820761e436c20d749c008f2e09320d74ac002f2e09320d71d06c712c2005230b0f2d089d74cd7393001a4e86c128407bbf2e093d74ac000f2e093ed55e2d20001c000915be0ebd72c08142091709601d72c081c12e25210b1e30f20d74a111213009601fa4001fa44f828fa443058baf2e091ed44d0810141d718f405049d7fc8ca0040048307f453f2e08b8e14038307f45bf2e08c22d70a00216e01b3b0f2d090e2c85003cf1612f400c9ed54007230d72c08248e2d21f2e092d200ed44d0d2005113baf2d08f54503091319c01810140d721d70a00f2e08ee2c8ca0058cf16c9ed5493f2c08de20010935bdb31e1d74cd0b4d6c35e";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TonAddress {
    pub workchain: i8,
    pub hash: [u8; 32],
    pub bounceable: Option<bool>,
    pub test_only: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cell {
    pub bits: Vec<bool>,
    pub refs: Vec<Cell>,
}

#[derive(Default)]
pub struct CellBuilder {
    bits: Vec<bool>,
    refs: Vec<Cell>,
}

#[derive(Clone, Debug)]
struct FlatCell {
    bits: Vec<bool>,
    refs: Vec<usize>,
}

#[derive(Clone, Debug)]
struct ParsedCell {
    bits: Vec<bool>,
    refs: Vec<usize>,
}

impl CellBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn store_bit(&mut self, value: bool) -> &mut Self {
        self.bits.push(value);
        self
    }

    pub fn store_bits(&mut self, bits: &[bool]) -> &mut Self {
        self.bits.extend_from_slice(bits);
        self
    }

    pub fn store_bytes(&mut self, bytes: &[u8]) -> &mut Self {
        for &b in bytes {
            self.store_uint(u64::from(b), 8);
        }
        self
    }

    pub fn store_uint(&mut self, value: u64, bits: usize) -> &mut Self {
        for i in (0..bits).rev() {
            self.bits.push(((value >> i) & 1) != 0);
        }
        self
    }

    pub fn store_u32(&mut self, value: u32) -> &mut Self {
        self.store_uint(u64::from(value), 32)
    }

    pub fn store_u64(&mut self, value: u64) -> &mut Self {
        self.store_uint(value, 64)
    }

    #[allow(dead_code)]
    pub fn store_big_uint(&mut self, value: &BigUint, bits: usize) -> WalletResult<&mut Self> {
        let bytes = value.to_bytes_be();
        if bytes.len() * 8 > bits {
            return Err(WalletError::invalid_input(
                "integer does not fit requested bit width",
            ));
        }
        let pad_bits = bits - bytes.len() * 8;
        for _ in 0..pad_bits {
            self.store_bit(false);
        }
        self.store_bytes(&bytes);
        Ok(self)
    }

    pub fn store_maybe_ref(&mut self, cell: Option<Cell>) -> &mut Self {
        match cell {
            Some(cell) => {
                self.store_bit(true);
                self.refs.push(cell);
            }
            None => {
                self.store_bit(false);
            }
        }
        self
    }

    pub fn store_ref(&mut self, cell: Cell) -> &mut Self {
        self.refs.push(cell);
        self
    }

    pub fn store_coins(&mut self, amount: &BigUint) -> WalletResult<&mut Self> {
        // VarUInteger 16: 4-bit length in bytes, followed by value bytes.
        if amount == &BigUint::from(0u8) {
            self.store_uint(0, 4);
            return Ok(self);
        }
        let bytes = amount.to_bytes_be();
        if bytes.len() > 15 {
            return Err(WalletError::invalid_input("TON coin value is too large"));
        }
        self.store_uint(bytes.len() as u64, 4);
        self.store_bytes(&bytes);
        Ok(self)
    }

    pub fn store_msg_address(&mut self, address: Option<&TonAddress>) -> WalletResult<&mut Self> {
        match address {
            None => {
                // addr_none$00 = MsgAddressExt / MsgAddress
                self.store_uint(0, 2);
            }
            Some(addr) => {
                // addr_std$10 anycast:(Maybe Anycast) workchain_id:int8 address:bits256
                self.store_uint(0b10, 2);
                self.store_bit(false); // no anycast
                self.store_int8(addr.workchain);
                self.store_bytes(&addr.hash);
            }
        }
        Ok(self)
    }

    pub fn store_int8(&mut self, value: i8) -> &mut Self {
        self.store_uint(value as u8 as u64, 8)
    }

    pub fn store_string_tail(&mut self, text: &str) -> &mut Self {
        self.store_bytes(text.as_bytes())
    }

    pub fn build(self) -> WalletResult<Cell> {
        if self.bits.len() > 1023 {
            return Err(WalletError::Internal(format!(
                "TON cell bit length {} exceeds 1023",
                self.bits.len()
            )));
        }
        if self.refs.len() > 4 {
            return Err(WalletError::Internal(format!(
                "TON cell ref count {} exceeds 4",
                self.refs.len()
            )));
        }
        Ok(Cell {
            bits: self.bits,
            refs: self.refs,
        })
    }
}

pub fn begin_cell() -> CellBuilder {
    CellBuilder::new()
}

pub fn parse_ton_address(input: &str) -> WalletResult<TonAddress> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(WalletError::invalid_input("TON address is required"));
    }
    if let Some((wc, hash)) = parse_raw_ton_address(trimmed)? {
        return Ok(TonAddress {
            workchain: wc,
            hash,
            bounceable: None,
            test_only: false,
        });
    }
    parse_user_friendly_address(trimmed)
}

pub fn format_user_friendly_address(
    address: &TonAddress,
    bounceable: bool,
    test_only: bool,
) -> String {
    let mut payload = [0u8; 36];
    let mut tag = if bounceable { 0x11u8 } else { 0x51u8 };
    if test_only {
        tag |= 0x80;
    }
    payload[0] = tag;
    payload[1] = address.workchain as u8;
    payload[2..34].copy_from_slice(&address.hash);
    let crc = crc16_xmodem(&payload[..34]);
    payload[34] = (crc >> 8) as u8;
    payload[35] = (crc & 0xff) as u8;
    base64_encode_url_nopad(&payload)
}

pub fn format_raw_ton_address(address: &TonAddress) -> String {
    format!("{}:{}", address.workchain, hex_encode(&address.hash))
}

pub fn wallet_v4r2_code_cell() -> WalletResult<Cell> {
    let boc = decode_hex(TON_WALLET_V4R2_CODE_BOC_HEX)?;
    parse_boc_single_root(&boc)
}

#[allow(dead_code)]
pub fn wallet_v4r2_code_boc_base64() -> WalletResult<String> {
    let boc = decode_hex(TON_WALLET_V4R2_CODE_BOC_HEX)?;
    Ok(base64_encode_std_nopad(&boc))
}

pub fn wallet_v4r2_data_cell(pubkey32: &[u8; 32], wallet_id: u32) -> WalletResult<Cell> {
    let mut b = begin_cell();
    b.store_u32(0); // seqno
    b.store_u32(wallet_id);
    b.store_bytes(pubkey32);
    // plugins: HashmapE 256 SimpleLib, empty dictionary => 0 bit
    b.store_bit(false);
    b.build()
}

pub fn state_init_cell(code: Cell, data: Cell) -> WalletResult<Cell> {
    let mut b = begin_cell();
    b.store_bit(false); // split_depth none
    b.store_bit(false); // special none
    b.store_maybe_ref(Some(code));
    b.store_maybe_ref(Some(data));
    b.store_bit(false); // library empty HashmapE
    b.build()
}

pub fn contract_address_from_state_init(state_init: &Cell, workchain: i8) -> TonAddress {
    TonAddress {
        workchain,
        hash: cell_hash(state_init),
        bounceable: None,
        test_only: false,
    }
}

pub fn build_comment_body(comment: &str) -> WalletResult<Cell> {
    let mut b = begin_cell();
    b.store_u32(0);
    b.store_string_tail(comment);
    b.build()
}

pub fn build_internal_message(
    dest: &TonAddress,
    amount_nanotons: &BigUint,
    bounce: bool,
    body: Option<Cell>,
) -> WalletResult<Cell> {
    let mut b = begin_cell();
    // int_msg_info$0 ihr_disabled:Bool bounce:Bool bounced:Bool src:MsgAddress ...
    b.store_bit(false); // tag = int_msg_info$0
    b.store_bit(true); // ihr_disabled
    b.store_bit(bounce);
    b.store_bit(false); // bounced
    b.store_msg_address(None)?; // src: addr_none
    b.store_msg_address(Some(dest))?;
    b.store_coins(amount_nanotons)?;
    b.store_bit(false); // extra currencies dict empty
    b.store_coins(&BigUint::from(0u8))?; // ihr_fee
    b.store_coins(&BigUint::from(0u8))?; // fwd_fee
    b.store_u64(0); // created_lt
    b.store_u32(0); // created_at
    b.store_bit(false); // init none
    match body {
        Some(body) => {
            b.store_bit(true); // body in ref
            b.store_ref(body);
        }
        None => {
            b.store_bit(false); // inline empty body
        }
    }
    b.build()
}

pub fn build_jetton_transfer_body(
    amount_units: &BigUint,
    destination_owner: &TonAddress,
    response_destination: &TonAddress,
    forward_ton_amount: &BigUint,
    memo: Option<&str>,
) -> WalletResult<Cell> {
    let mut b = begin_cell();
    b.store_u32(TON_JETTON_OP_TRANSFER);
    b.store_u64(0); // query_id
    b.store_coins(amount_units)?;
    b.store_msg_address(Some(destination_owner))?;
    b.store_msg_address(Some(response_destination))?;
    b.store_bit(false); // custom_payload: none
    b.store_coins(forward_ton_amount)?;
    let forward_payload = if let Some(text) = memo.map(str::trim).filter(|s| !s.is_empty()) {
        build_comment_body(text)?
    } else {
        begin_cell().build()?
    };
    b.store_bit(true); // forward_payload in ref
    b.store_ref(forward_payload);
    b.build()
}

pub fn build_wallet_v4r2_signing_body(
    wallet_id: u32,
    valid_until: u32,
    seqno: u32,
    mode: u8,
    out_msg: Cell,
) -> WalletResult<Cell> {
    let mut b = begin_cell();
    b.store_u32(wallet_id);
    b.store_u32(valid_until);
    b.store_u32(seqno);
    b.store_u32(0); // opcode = 0 (simple send)
    b.store_uint(u64::from(mode), 8);
    b.store_ref(out_msg);
    b.build()
}

pub fn build_wallet_v4r2_body_with_signature(
    signature64: &[u8],
    signing_body: &Cell,
) -> WalletResult<Cell> {
    if signature64.len() != 64 {
        return Err(WalletError::invalid_input(
            "TON wallet signature must be 64 bytes",
        ));
    }
    let mut b = begin_cell();
    b.store_bytes(signature64);
    b.store_bits(&signing_body.bits);
    for r in &signing_body.refs {
        b.store_ref(r.clone());
    }
    b.build()
}

pub fn build_external_message(
    wallet_address: &TonAddress,
    body: Cell,
    state_init: Option<Cell>,
) -> WalletResult<Cell> {
    let mut b = begin_cell();
    // ext_in_msg_info$10 src:MsgAddressExt dest:MsgAddressInt import_fee:Grams
    b.store_uint(0b10, 2);
    b.store_msg_address(None)?; // src addr_none
    b.store_msg_address(Some(wallet_address))?;
    b.store_coins(&BigUint::from(0u8))?;
    match state_init {
        Some(init) => {
            b.store_bit(true); // init exists
            b.store_bit(true); // init as ref
            b.store_ref(init);
        }
        None => {
            b.store_bit(false); // no init
        }
    }
    b.store_bit(true); // body as ref
    b.store_ref(body);
    b.build()
}

pub fn cell_hash(cell: &Cell) -> [u8; 32] {
    let mut repr = Vec::new();
    repr.push(cell_descriptor_1(cell));
    repr.push(cell_descriptor_2(cell.bits.len()));
    repr.extend(bits_to_padded_bytes(&cell.bits));

    for r in &cell.refs {
        let depth = cell_depth(r);
        repr.push((depth >> 8) as u8);
        repr.push((depth & 0xff) as u8);
    }
    for r in &cell.refs {
        repr.extend_from_slice(&cell_hash(r));
    }

    let mut hasher = Sha256::new();
    hasher.update(&repr);
    let out = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&out);
    hash
}

pub fn cell_to_boc_bytes(cell: &Cell) -> WalletResult<Vec<u8>> {
    let mut flat = Vec::<FlatCell>::new();
    let root_idx = flatten_cell(cell, &mut flat);

    let cells_data = serialize_cells_data(&flat)?;
    let cells_num = flat.len();
    let roots_num = 1usize;
    let absent_num = 0usize;
    let total_cells_size = cells_data.len();

    let size_bytes = minimal_be_bytes_len(cells_num.saturating_sub(1).max(1) as u64);
    let offset_bytes = minimal_be_bytes_len(total_cells_size.max(1) as u64);

    let mut out = Vec::with_capacity(32 + cells_data.len());
    out.extend_from_slice(&[0xb5, 0xee, 0x9c, 0x72]);
    let flags = (size_bytes as u8) & 0x07; // no idx, no crc32, no cache bits
    out.push(flags);
    out.push(offset_bytes as u8);
    write_be_var(&mut out, cells_num as u64, size_bytes)?;
    write_be_var(&mut out, roots_num as u64, size_bytes)?;
    write_be_var(&mut out, absent_num as u64, size_bytes)?;
    write_be_var(&mut out, total_cells_size as u64, offset_bytes)?;
    write_be_var(&mut out, root_idx as u64, size_bytes)?;
    out.extend_from_slice(&cells_data);
    Ok(out)
}

pub fn cell_to_boc_base64(cell: &Cell) -> WalletResult<String> {
    Ok(base64_encode_std_nopad(&cell_to_boc_bytes(cell)?))
}

pub fn parse_boc_single_root(bytes: &[u8]) -> WalletResult<Cell> {
    if bytes.len() < 8 || &bytes[..4] != [0xb5, 0xee, 0x9c, 0x72] {
        return Err(WalletError::invalid_input("invalid TON BOC magic"));
    }
    let flags = bytes[4];
    let has_idx = (flags & 0x80) != 0;
    let has_crc32 = (flags & 0x40) != 0;
    let _has_cache_bits = (flags & 0x20) != 0;
    let size_bytes = usize::from(flags & 0x07);
    if size_bytes == 0 || size_bytes > 8 {
        return Err(WalletError::invalid_input("invalid TON BOC size bytes"));
    }
    let offset_bytes = bytes[5] as usize;
    if offset_bytes == 0 || offset_bytes > 8 {
        return Err(WalletError::invalid_input("invalid TON BOC offset bytes"));
    }
    let mut p = 6usize;
    let cells_num = read_be_var(bytes, &mut p, size_bytes)? as usize;
    let roots_num = read_be_var(bytes, &mut p, size_bytes)? as usize;
    let _absent_num = read_be_var(bytes, &mut p, size_bytes)? as usize;
    let total_cells_size = read_be_var(bytes, &mut p, offset_bytes)? as usize;
    if roots_num != 1 {
        return Err(WalletError::invalid_input(
            "only single-root TON BOC is supported",
        ));
    }
    let root_index = read_be_var(bytes, &mut p, size_bytes)? as usize;
    if root_index >= cells_num {
        return Err(WalletError::invalid_input(
            "TON BOC root index out of range",
        ));
    }
    if has_idx {
        let skip = cells_num
            .checked_mul(offset_bytes)
            .ok_or_else(|| WalletError::Internal("TON BOC index table overflow".into()))?;
        p = p
            .checked_add(skip)
            .ok_or_else(|| WalletError::Internal("TON BOC parse overflow".into()))?;
    }
    let data_start = p;
    let data_end = data_start
        .checked_add(total_cells_size)
        .ok_or_else(|| WalletError::Internal("TON BOC size overflow".into()))?;
    if data_end > bytes.len() {
        return Err(WalletError::invalid_input("TON BOC truncated cells data"));
    }
    let mut parsed = Vec::with_capacity(cells_num);
    while p < data_end && parsed.len() < cells_num {
        let d1 = *bytes
            .get(p)
            .ok_or_else(|| WalletError::invalid_input("TON BOC truncated cell descriptor"))?;
        p += 1;
        let d2 = *bytes
            .get(p)
            .ok_or_else(|| WalletError::invalid_input("TON BOC truncated cell descriptor"))?;
        p += 1;
        let refs_count = usize::from(d1 & 0x07);
        let exotic = (d1 & 0x08) != 0;
        let level = d1 >> 5;
        if exotic {
            return Err(WalletError::invalid_input(
                "TON exotic cells are not supported in parser",
            ));
        }
        if level != 0 {
            return Err(WalletError::invalid_input(
                "TON non-zero level cells are not supported in parser",
            ));
        }
        let full_bytes = usize::from(d2 / 2);
        let has_partial = (d2 % 2) != 0;
        let data_bytes_len = full_bytes + usize::from(has_partial);
        let data = bytes
            .get(p..p + data_bytes_len)
            .ok_or_else(|| WalletError::invalid_input("TON BOC truncated cell data"))?;
        p += data_bytes_len;
        let bits = unpadded_bytes_to_bits(data, full_bytes, has_partial)?;
        let mut refs = Vec::with_capacity(refs_count);
        for _ in 0..refs_count {
            let idx = read_be_var(bytes, &mut p, size_bytes)? as usize;
            refs.push(idx);
        }
        parsed.push(ParsedCell { bits, refs });
    }
    if parsed.len() != cells_num || p != data_end {
        return Err(WalletError::invalid_input(
            "TON BOC cells section parse mismatch",
        ));
    }
    if has_crc32 {
        let _ = bytes
            .get(data_end..data_end + 4)
            .ok_or_else(|| WalletError::invalid_input("TON BOC truncated crc32"))?;
    }
    let mut memo: Vec<Option<Cell>> = vec![None; parsed.len()];
    build_parsed_cell(root_index, &parsed, &mut memo)
}

fn build_parsed_cell(
    idx: usize,
    parsed: &[ParsedCell],
    memo: &mut [Option<Cell>],
) -> WalletResult<Cell> {
    if let Some(cell) = &memo[idx] {
        return Ok(cell.clone());
    }
    let p = parsed
        .get(idx)
        .ok_or_else(|| WalletError::invalid_input("TON BOC ref out of range"))?;
    let mut refs = Vec::with_capacity(p.refs.len());
    for &r in &p.refs {
        refs.push(build_parsed_cell(r, parsed, memo)?);
    }
    let cell = Cell {
        bits: p.bits.clone(),
        refs,
    };
    memo[idx] = Some(cell.clone());
    Ok(cell)
}

fn parse_raw_ton_address(input: &str) -> WalletResult<Option<(i8, [u8; 32])>> {
    let Some((wc_text, hash_text)) = input.split_once(':') else {
        return Ok(None);
    };
    let wc = wc_text
        .trim()
        .parse::<i32>()
        .map_err(|_| WalletError::invalid_input("invalid TON raw workchain id"))?;
    if !(i8::MIN as i32..=i8::MAX as i32).contains(&wc) {
        return Err(WalletError::invalid_input(
            "TON raw workchain id out of range",
        ));
    }
    let hash_bytes = decode_hex(hash_text.trim())?;
    if hash_bytes.len() != 32 {
        return Err(WalletError::invalid_input(
            "TON raw address hash must be 32 bytes hex",
        ));
    }
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&hash_bytes);
    Ok(Some((wc as i8, hash)))
}

fn parse_user_friendly_address(input: &str) -> WalletResult<TonAddress> {
    let decoded = base64_decode_url_or_std(input)?;
    if decoded.len() != 36 {
        return Err(WalletError::invalid_input(
            "TON user-friendly address must decode to 36 bytes",
        ));
    }
    let crc_expected = u16::from_be_bytes([decoded[34], decoded[35]]);
    let crc_actual = crc16_xmodem(&decoded[..34]);
    if crc_expected != crc_actual {
        return Err(WalletError::invalid_input("TON address crc16 mismatch"));
    }
    let tag = decoded[0];
    let test_only = (tag & 0x80) != 0;
    let bounceable = (tag & 0x40) == 0;
    let low = tag & 0x3f;
    if low != 0x11 {
        return Err(WalletError::invalid_input("unsupported TON address tag"));
    }
    let workchain = decoded[1] as i8;
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&decoded[2..34]);
    Ok(TonAddress {
        workchain,
        hash,
        bounceable: Some(bounceable),
        test_only,
    })
}

fn flatten_cell(cell: &Cell, flat: &mut Vec<FlatCell>) -> usize {
    let idx = flat.len();
    flat.push(FlatCell {
        bits: cell.bits.clone(),
        refs: Vec::new(),
    });
    let mut ref_ids = Vec::with_capacity(cell.refs.len());
    for r in &cell.refs {
        let rid = flatten_cell(r, flat);
        ref_ids.push(rid);
    }
    flat[idx].refs = ref_ids;
    idx
}

fn serialize_cells_data(flat: &[FlatCell]) -> WalletResult<Vec<u8>> {
    let size_bytes = minimal_be_bytes_len(flat.len().saturating_sub(1).max(1) as u64);
    let mut out = Vec::new();
    for cell in flat {
        if cell.refs.len() > 4 {
            return Err(WalletError::Internal("TON BOC serialize: refs > 4".into()));
        }
        out.push((cell.refs.len() as u8) & 0x07); // ordinary, level 0
        out.push(cell_descriptor_2(cell.bits.len()));
        out.extend(bits_to_padded_bytes(&cell.bits));
        for &r in &cell.refs {
            write_be_var(&mut out, r as u64, size_bytes)?;
        }
    }
    Ok(out)
}

fn cell_descriptor_1(cell: &Cell) -> u8 {
    (cell.refs.len() as u8) & 0x07
}

fn cell_descriptor_2(bits_len: usize) -> u8 {
    let full_bytes = bits_len / 8;
    let partial = usize::from(!bits_len.is_multiple_of(8));
    (full_bytes + full_bytes + partial) as u8
}

fn bits_to_padded_bytes(bits: &[bool]) -> Vec<u8> {
    if bits.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(bits.len().div_ceil(8));
    let mut i = 0usize;
    while i < bits.len() {
        let end = (i + 8).min(bits.len());
        let chunk = &bits[i..end];
        let mut byte = 0u8;
        for (j, bit) in chunk.iter().enumerate() {
            if *bit {
                byte |= 1 << (7 - j);
            }
        }
        if chunk.len() < 8 {
            // End-bit padding: append single 1 bit, then zeros.
            byte |= 1 << (7 - chunk.len());
        }
        out.push(byte);
        i += 8;
    }
    out
}

fn unpadded_bytes_to_bits(
    data: &[u8],
    full_bytes: usize,
    has_partial: bool,
) -> WalletResult<Vec<bool>> {
    if data.len() != full_bytes + usize::from(has_partial) {
        return Err(WalletError::invalid_input("TON cell data length mismatch"));
    }
    let mut bits = Vec::with_capacity(full_bytes * 8 + if has_partial { 7 } else { 0 });
    for &b in &data[..full_bytes] {
        for i in 0..8 {
            bits.push(((b >> (7 - i)) & 1) != 0);
        }
    }
    if has_partial {
        let last = *data.last().unwrap_or(&0);
        if last == 0 {
            return Err(WalletError::invalid_input(
                "TON partial cell byte has no terminator",
            ));
        }
        let tz = last.trailing_zeros() as usize;
        if tz > 6 {
            return Err(WalletError::invalid_input(
                "TON partial cell padding is invalid",
            ));
        }
        let data_bits_in_last = 7usize.saturating_sub(tz);
        for i in 0..data_bits_in_last {
            bits.push(((last >> (7 - i)) & 1) != 0);
        }
    }
    Ok(bits)
}

fn cell_depth(cell: &Cell) -> u16 {
    if cell.refs.is_empty() {
        0
    } else {
        let max_depth = cell.refs.iter().map(cell_depth).max().unwrap_or(0);
        max_depth.saturating_add(1)
    }
}

fn minimal_be_bytes_len(value: u64) -> usize {
    if value <= 0xff {
        1
    } else if value <= 0xffff {
        2
    } else if value <= 0xff_ffff {
        3
    } else if value <= 0xffff_ffff {
        4
    } else if value <= 0xff_ff_ff_ff_ff {
        5
    } else if value <= 0xffff_ffff_ffff {
        6
    } else if value <= 0xff_ff_ff_ff_ff_ff_ff {
        7
    } else {
        8
    }
}

fn write_be_var(out: &mut Vec<u8>, value: u64, width: usize) -> WalletResult<()> {
    if width == 0 || width > 8 {
        return Err(WalletError::Internal("invalid write_be_var width".into()));
    }
    let max = if width == 8 {
        u64::MAX
    } else {
        (1u64 << (width * 8)) - 1
    };
    if value > max {
        return Err(WalletError::Internal(format!(
            "value {value} does not fit in {width} bytes"
        )));
    }
    for i in (0..width).rev() {
        out.push(((value >> (i * 8)) & 0xff) as u8);
    }
    Ok(())
}

fn read_be_var(data: &[u8], p: &mut usize, width: usize) -> WalletResult<u64> {
    let slice = data
        .get(*p..*p + width)
        .ok_or_else(|| WalletError::invalid_input("TON BOC truncated integer"))?;
    *p += width;
    let mut out = 0u64;
    for &b in slice {
        out = (out << 8) | u64::from(b);
    }
    Ok(out)
}

pub fn base64_encode_std_nopad(data: &[u8]) -> String {
    base64_encode(data, BASE64_STD_ALPHABET, false)
}

pub fn base64_encode_url_nopad(data: &[u8]) -> String {
    base64_encode(data, BASE64_URL_ALPHABET, false)
}

fn base64_encode(data: &[u8], alphabet: &[u8; 64], pad: bool) -> String {
    if data.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    let mut i = 0usize;
    while i + 3 <= data.len() {
        let n = (u32::from(data[i]) << 16) | (u32::from(data[i + 1]) << 8) | u32::from(data[i + 2]);
        out.push(alphabet[((n >> 18) & 0x3f) as usize] as char);
        out.push(alphabet[((n >> 12) & 0x3f) as usize] as char);
        out.push(alphabet[((n >> 6) & 0x3f) as usize] as char);
        out.push(alphabet[(n & 0x3f) as usize] as char);
        i += 3;
    }
    match data.len() - i {
        1 => {
            let n = u32::from(data[i]) << 16;
            out.push(alphabet[((n >> 18) & 0x3f) as usize] as char);
            out.push(alphabet[((n >> 12) & 0x3f) as usize] as char);
            if pad {
                out.push('=');
                out.push('=');
            }
        }
        2 => {
            let n = (u32::from(data[i]) << 16) | (u32::from(data[i + 1]) << 8);
            out.push(alphabet[((n >> 18) & 0x3f) as usize] as char);
            out.push(alphabet[((n >> 12) & 0x3f) as usize] as char);
            out.push(alphabet[((n >> 6) & 0x3f) as usize] as char);
            if pad {
                out.push('=');
            }
        }
        _ => {}
    }
    out
}

fn base64_decode_url_or_std(text: &str) -> WalletResult<Vec<u8>> {
    let mut filtered = text.trim().as_bytes().to_vec();
    filtered.retain(|b| !b" \n\r\t".contains(b));
    while filtered.len() % 4 != 0 {
        filtered.push(b'=');
    }
    let mut out = Vec::with_capacity(filtered.len() / 4 * 3);
    let mut i = 0usize;
    while i < filtered.len() {
        let c0 = filtered[i];
        let c1 = filtered[i + 1];
        let c2 = filtered[i + 2];
        let c3 = filtered[i + 3];
        i += 4;
        let v0 = b64_val(c0)?;
        let v1 = b64_val(c1)?;
        let v2 = if c2 == b'=' { 0 } else { b64_val(c2)? };
        let v3 = if c3 == b'=' { 0 } else { b64_val(c3)? };
        let n =
            (u32::from(v0) << 18) | (u32::from(v1) << 12) | (u32::from(v2) << 6) | u32::from(v3);
        out.push(((n >> 16) & 0xff) as u8);
        if c2 != b'=' {
            out.push(((n >> 8) & 0xff) as u8);
        }
        if c3 != b'=' {
            out.push((n & 0xff) as u8);
        }
    }
    Ok(out)
}

fn b64_val(c: u8) -> WalletResult<u8> {
    match c {
        b'A'..=b'Z' => Ok(c - b'A'),
        b'a'..=b'z' => Ok(c - b'a' + 26),
        b'0'..=b'9' => Ok(c - b'0' + 52),
        b'+' | b'-' => Ok(62),
        b'/' | b'_' => Ok(63),
        b'=' => Ok(0),
        _ => Err(WalletError::invalid_input("invalid base64 character")),
    }
}

fn crc16_xmodem(data: &[u8]) -> u16 {
    let mut crc = 0u16;
    for &b in data {
        crc ^= u16::from(b) << 8;
        for _ in 0..8 {
            if (crc & 0x8000) != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

pub fn decode_hex(input: &str) -> WalletResult<Vec<u8>> {
    let s = input.trim();
    if !s.len().is_multiple_of(2) {
        return Err(WalletError::invalid_input("hex length must be even"));
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let hi = hex_nibble(bytes[i])?;
        let lo = hex_nibble(bytes[i + 1])?;
        out.push((hi << 4) | lo);
        i += 2;
    }
    Ok(out)
}

pub fn hex_encode(data: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(data.len() * 2);
    for &b in data {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

fn hex_nibble(c: u8) -> WalletResult<u8> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(WalletError::invalid_input("invalid hex character")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ton_friendly_roundtrip() {
        let addr = TonAddress {
            workchain: 0,
            hash: [7u8; 32],
            bounceable: Some(false),
            test_only: false,
        };
        let text = format_user_friendly_address(&addr, false, false);
        let parsed = parse_ton_address(&text).expect("parse");
        assert_eq!(parsed.workchain, 0);
        assert_eq!(parsed.hash, [7u8; 32]);
        assert_eq!(parsed.bounceable, Some(false));
    }

    #[test]
    fn parses_wallet_code_boc() {
        let code = wallet_v4r2_code_cell().expect("code boc parse");
        assert!(!code.bits.is_empty());
    }

    #[test]
    fn cell_boc_roundtrip_single() {
        let mut b = begin_cell();
        b.store_u32(0x12345678);
        let cell = b.build().unwrap();
        let boc = cell_to_boc_bytes(&cell).unwrap();
        let parsed = parse_boc_single_root(&boc).unwrap();
        assert_eq!(parsed.bits, cell.bits);
        assert_eq!(parsed.refs.len(), 0);
    }
}

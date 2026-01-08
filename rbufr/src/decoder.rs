#[allow(unused)]
use crate::{
    block::MessageBlock,
    errors::{Error, Result},
    structs::versions::MessageVersion,
    tables::{LocalTable, TableLoader},
};
#[cfg(feature = "opera")]
use genlib::tables::ArchivedBitMapEntry;
use genlib::{
    ArchivedFXY, BUFRKey, FXY,
    prelude::{BUFRTableB, BUFRTableBitMap, BUFRTableD},
    tables::{ArchivedBTableEntry, ArchivedDTableEntry},
};
use std::{borrow::Cow, fmt::Display, ops::Deref};

const MISS_VAL: f64 = 99999.999999;

pub struct Decoder {
    #[allow(unused)]
    bufr_edition: u8,
    master_b: BUFRTableB,
    master_d: BUFRTableD,
    // local
    local_b: Option<BUFRTableB>,
    local_d: Option<BUFRTableD>,
    // opera
    #[cfg(feature = "opera")]
    opera_bitmap_table: Option<BUFRTableBitMap>,
}

struct Cache<'a> {
    master_b: &'a BUFRTableB,
    master_d: &'a BUFRTableD,
    local_b: Option<&'a BUFRTableB>,
    local_d: Option<&'a BUFRTableD>,
}

impl<'a> Cache<'a> {
    fn new(
        master_b: &'a BUFRTableB,
        master_d: &'a BUFRTableD,
        local_b: Option<&'a BUFRTableB>,
        local_d: Option<&'a BUFRTableD>,
    ) -> Self {
        Self {
            master_b,
            master_d,
            local_b,
            local_d,
        }
    }

    #[inline(always)]
    fn get_b<K: BUFRKey>(&mut self, fxy: &K) -> Option<&'a ArchivedBTableEntry> {
        self.lookup_b_descriptor(fxy)
    }

    #[inline(always)]
    fn get_d<K: BUFRKey>(&mut self, fxy: &K) -> Option<&'a ArchivedDTableEntry> {
        self.lookup_d_descriptor(fxy)
    }

    #[inline(always)]
    fn lookup_b_descriptor<K: BUFRKey>(&self, fxy: &K) -> Option<&'a ArchivedBTableEntry> {
        self.lookup_local_b_descriptor(fxy)
            .or_else(|| self.lookup_master_b_descriptor(fxy))
    }

    #[inline]
    fn lookup_local_b_descriptor<K: BUFRKey>(&self, fxy: &K) -> Option<&'a ArchivedBTableEntry> {
        self.local_b
            .as_ref()
            .and_then(|t| t.lookup(fxy))
            .filter(|e| &e.fxy == fxy)
    }

    #[inline]
    fn lookup_master_b_descriptor<K: BUFRKey>(&self, fxy: &K) -> Option<&'a ArchivedBTableEntry> {
        self.master_b.lookup(fxy).filter(|e| &e.fxy == fxy)
    }

    #[inline]
    fn lookup_master_d_descriptor<K: BUFRKey>(&self, fxy: &K) -> Option<&'a ArchivedDTableEntry> {
        self.master_d.lookup(fxy).filter(|e| &e.fxy == fxy)
    }

    #[inline]
    fn lookup_local_d_descriptor<K: BUFRKey>(&self, fxy: &K) -> Option<&'a ArchivedDTableEntry> {
        self.local_d
            .as_ref()
            .and_then(|t| t.lookup(fxy))
            .filter(|e| &e.fxy == fxy)
    }

    #[inline(always)]
    fn lookup_d_descriptor<K: BUFRKey>(&self, fxy: &K) -> Option<&'a ArchivedDTableEntry> {
        self.lookup_local_d_descriptor(fxy)
            .or_else(|| self.lookup_master_d_descriptor(fxy))
    }
}

struct State {
    // Common State
    common_scale: Option<i32>,
    common_ref_value: Option<i32>,
    common_data_width: Option<i32>,
    common_str_width: Option<usize>,
    // Localized State
    local_data_width: Option<i32>,
    // Temporary storage
    temp_operator: Option<i32>,
}

/// Pre-compiled metadata for one field in the array body
#[derive(Debug, Clone)]
struct FieldSpec<'a> {
    /// Original FXY for debugging/output
    fxy: FXY,
    /// Name from Table B
    name: &'a str,
    /// Unit from Table B
    unit: &'a str,
    /// Effective bit width (after operators applied)
    width_bits: u32,
    /// Effective scale (after operators applied)
    scale: i32,
    /// Effective reference value (after operators applied)
    reference: i32,
    /// Missing value for this field (all bits set for this width)
    missing_value: u64,
}

/// Compiled layout for one array repetition
#[derive(Debug, Clone)]
struct CompiledLayout<'a> {
    fields: Vec<FieldSpec<'a>>,
    bits_per_element: usize,
}

#[derive(Debug)]
struct CompilerState {
    common_scale: Option<i32>,
    common_ref_value: Option<i32>,
    common_data_width: Option<i32>,
    temp_operator: Option<i32>,
    common_str_width: Option<usize>,
    local_data_width: Option<i32>,
}

impl State {
    fn new() -> Self {
        Self {
            common_scale: None,
            common_ref_value: None,
            common_data_width: None,
            common_str_width: None,
            local_data_width: None,
            temp_operator: None,
        }
    }

    #[inline(always)]
    fn no_change(&self, e: &ArchivedBTableEntry) -> bool {
        let unit = e.bufr_unit.as_str();
        let is_flag_or_code = matches!(
            unit,
            "flag table" | "flag-table" | "code table" | "code-table"
        );
        let delay_repeat_count = e.fxy.f.to_native() == 0 && e.fxy.x.to_native() == 31;

        is_flag_or_code || delay_repeat_count
    }

    #[inline(always)]
    fn datawidth(&self, e: &ArchivedBTableEntry) -> u32 {
        if let Some(local_width) = self.local_data_width {
            return local_width as u32;
        }

        let v = if self.no_change(e) {
            e.bufr_datawidth_bits.to_native()
        } else {
            self.common_data_width
                .map(|c| {
                    let (v, _) = e
                        .bufr_datawidth_bits
                        .to_native()
                        .overflowing_add_signed(c - 128);
                    v
                })
                .unwrap_or(e.bufr_datawidth_bits.to_native())
        };

        if let Some(op) = self.temp_operator {
            v + (10 * op) as u32
        } else {
            v
        }
    }

    #[inline(always)]
    fn scale(&self, e: &ArchivedBTableEntry) -> i32 {
        let v = if self.no_change(e) {
            e.bufr_scale.to_native()
        } else {
            self.common_scale
                .map(|c| {
                    let (v, _) = e.bufr_scale.to_native().overflowing_add(128 - c);
                    v
                })
                .unwrap_or(e.bufr_scale.to_native())
        };

        if let Some(op) = self.temp_operator {
            e.bufr_scale.to_native() + op
        } else {
            v
        }
    }

    #[inline(always)]
    fn reference_value(&self, e: &ArchivedBTableEntry) -> i32 {
        let v = e.bufr_reference_value.to_native();

        if let Some(op) = self.temp_operator {
            (v as f32 * 10_f32.powi(op)) as i32
        } else {
            v
        }
    }
}

impl Decoder {
    pub fn from_message(message: &MessageBlock) -> Result<Self> {
        let table_info = message.table_info();
        let master_table_version = table_info.master_table_version;

        let master_b: BUFRTableB = message.load_first_validable_table(master_table_version)?;
        let master_d: BUFRTableD = message.load_first_validable_table(master_table_version)?;

        let local_table_version = table_info.local_table_version as u32;

        let local_tables = if local_table_version > 0 {
            let local_b: BUFRTableB = TableLoader.load_table(LocalTable::new(
                Some(table_info.subcenter_id * 256 + table_info.center_id),
                table_info.local_table_version,
            ))?;

            let local_d: BUFRTableD = TableLoader.load_table(LocalTable::new(
                Some(table_info.subcenter_id * 256 + table_info.center_id),
                table_info.local_table_version,
            ))?;

            Some((local_b, local_d))
        } else {
            None
        };

        let (local_b, local_d) = if let Some((b, d)) = local_tables {
            (Some(b), Some(d))
        } else {
            (None, None)
        };

        #[cfg(feature = "opera")]
        let opera_bitmap_table = message
            .load_opera_bitmap_table(
                table_info.center_id,
                table_info.subcenter_id,
                table_info.local_table_version,
                master_table_version,
            )
            .ok();

        let decoder = Self::new(
            message.version(),
            master_b,
            master_d,
            local_b,
            local_d,
            #[cfg(feature = "opera")]
            opera_bitmap_table,
        );

        Ok(decoder)
    }

    pub fn new(
        edition: u8,
        master_b: BUFRTableB,
        master_d: BUFRTableD,
        local_b: Option<BUFRTableB>,
        local_d: Option<BUFRTableD>,

        #[cfg(feature = "opera")] _opera_bitmap_table: Option<BUFRTableBitMap>,
    ) -> Self {
        Decoder {
            bufr_edition: edition,
            master_b,
            master_d,
            local_b,
            local_d,
            #[cfg(feature = "opera")]
            opera_bitmap_table: _opera_bitmap_table,
        }
    }

    pub fn decode<'a, V: MessageVersion>(
        &'a mut self,
        message: &impl Deref<Target = V>,
    ) -> Result<BUFRParsed<'a>> {
        let data_block = message.data_block()?;
        let descriptors = message.descriptors()?;

        let mut data_input = BitInput::new(data_block);
        let mut record = BUFRParsed::new();
        let mut state = State::new();
        let mut cache = Cache::new(
            &self.master_b,
            &self.master_d,
            self.local_b.as_ref(),
            self.local_d.as_ref(),
        );

        let mut stack: Vec<Frame> = vec![];
        stack.push(Frame::Slice {
            descs: Descs::Raw(&descriptors),
            idx: 0,
        });

        while let Some(frame) = stack.pop() {
            match frame {
                Frame::Slice { descs, idx } => {
                    if idx >= descs.len() {
                        continue;
                    }
                    match descs {
                        Descs::Raw(raw) => {
                            let des = &raw[idx];
                            self.parse_slice(
                                des,
                                idx,
                                &mut record,
                                descs,
                                &mut stack,
                                &mut cache,
                                &mut state,
                                &mut data_input,
                            )?;
                        }
                        Descs::Archived(archived) => {
                            let des = &archived[idx];
                            self.parse_slice(
                                des,
                                idx,
                                &mut record,
                                descs,
                                &mut stack,
                                &mut cache,
                                &mut state,
                                &mut data_input,
                            )?;
                        }
                    }
                }

                Frame::Repeat {
                    descs,
                    times,
                    current,
                } => {
                    self.parse_repeating(times, current, descs, &mut stack)?;
                }

                Frame::CompiledArray { layout, times } => {
                    self.parse_compiled_array(&layout, times, &mut data_input, &mut record)?;
                }
            }
        }

        Ok(record)
    }

    #[inline]
    fn parse_slice<'k, 'c, 'i, 's, K: BUFRKey>(
        &self,
        des: &K,
        idx: usize,
        values: &mut BUFRParsed<'c>,
        descs: Descs<'k>,
        // Stack
        stack: &mut Vec<Frame<'k, 'c>>,
        cache: &mut Cache<'c>,
        state: &mut State,
        data: &mut BitInput<'i>,
    ) -> Result<()>
    where
        'c: 'k,
    {
        match des.f() {
            0 => {
                // Element descriptor - parse data
                if let Some(e) = cache.get_b(des) {
                    let value = self.evalute(state, data, &e)?;
                    values.push(value, e.element_name_en.as_str(), e.bufr_unit.as_str());
                    state.temp_operator = None;
                    state.local_data_width = None;

                    stack.push(Frame::Slice {
                        descs,
                        idx: idx + 1,
                    });
                } else {
                    return Err(Error::ParseError(format!(
                        "Descriptor {:?} not found in Table B",
                        des
                    )));
                }
            }
            1 => {
                let x = des.x() as usize;
                let mut y = des.y() as usize;
                let delay_repeat = y == 0;

                if delay_repeat {
                    let count = match descs {
                        Descs::Raw(raw) => {
                            let count_des = &raw[idx + 1];
                            self.parse_usize(state, cache, count_des, data)?
                        }

                        Descs::Archived(archived) => {
                            let count_des = &archived[idx + 1];
                            self.parse_usize(state, cache, count_des, data)?
                        }
                    };
                    y = count;
                }

                let body_start = if delay_repeat { idx + 2 } else { idx + 1 };
                let body_end = body_start + x;

                if body_end > descs.len() {
                    return Err(Error::ParseError(format!(
                        "Not enough descriptors to repeat: requested {}, available {}",
                        x,
                        descs.len() - body_start
                    )));
                }

                let compiled_layout = match descs {
                    Descs::Raw(raw) => {
                        let body = &raw[body_start..body_end];
                        self.try_compile_array_layout(body, y, cache)?
                    }
                    Descs::Archived(archived) => {
                        let body = &archived[body_start..body_end];
                        self.try_compile_array_layout(body, y, cache)?
                    }
                };

                stack.push(Frame::Slice {
                    descs,
                    idx: body_end,
                });

                let frame = if let Some(layout) = compiled_layout {
                    Frame::CompiledArray { layout, times: y }
                } else {
                    // Fallback to normal interpretation
                    match descs {
                        Descs::Raw(raw) => Frame::Repeat {
                            descs: Descs::Raw(&raw[body_start..body_end]),
                            times: y,
                            current: 0,
                        },
                        Descs::Archived(archived) => Frame::Repeat {
                            descs: Descs::Archived(&archived[body_start..body_end]),
                            times: y,
                            current: 0,
                        },
                    }
                };

                stack.push(frame);
            }
            2 => {
                self.deal_with_operator(state, values, des, data)?;
                stack.push(Frame::Slice {
                    descs,
                    idx: idx + 1,
                });
            }
            3 => {
                #[cfg(feature = "opera")]
                let opera_dw = self.parse_opera_bitmap(des).map(|e| e.depth);

                if let Some(seq) = cache.get_d(des) {
                    let fxy_chain = seq.fxy_chain.as_slice();
                    #[cfg(feature = "opera")]
                    if opera_dw.is_some() {
                        // TODO
                        unimplemented!("");
                    }

                    stack.push(Frame::Slice {
                        descs,
                        idx: idx + 1,
                    });

                    stack.push(Frame::Slice {
                        descs: Descs::Archived(fxy_chain),
                        idx: 0,
                    });
                } else {
                    return Err(Error::ParseError(format!(
                        "Sequence descriptor {:?} not found in Table D",
                        des
                    )));
                }
            }
            _ => {
                return Err(Error::ParseError(format!(
                    "Invalid descriptor F value: {}",
                    des.f()
                )));
            }
        }

        Ok(())
    }

    #[inline]
    fn _parse_slice<'c, 'i, 's, K: BUFRKey>(
        &self,
        des: &K,
        values: &mut BUFRParsed<'c>,
        // Stack
        cache: &mut Cache<'c>,
        state: &mut State,
        data: &mut BitInput<'i>,
    ) -> Result<()> {
        match des.f() {
            0 => {
                if let Some(e) = cache.get_b(des) {
                    let value = self.evalute(state, data, &e)?;
                    values.push(value, e.element_name_en.as_str(), e.bufr_unit.as_str());

                    state.temp_operator = None;
                    state.local_data_width = None;
                } else {
                    return Err(Error::ParseError(format!(
                        "Descriptor {:?} not found in Table B",
                        des
                    )));
                }
            }
            2 => {
                self.deal_with_operator(state, values, des, data)?;
            }
            _ => {
                return Err(Error::ParseError(format!(
                    "Invalid descriptor F value: {}",
                    des.f()
                )));
            }
        }

        Ok(())
    }

    fn parse_repeating<'k, 'c, 'i, 's>(
        &self,
        times: usize,
        current: usize,
        //
        descs: Descs<'k>,
        // Stack
        stack: &mut Vec<Frame<'k, '_>>,
    ) -> Result<()>
    where
        'c: 'k,
    {
        if current >= times {
            return Ok(());
        }
        stack.push(Frame::Repeat {
            descs,
            times,
            current: current + 1,
        });

        stack.push(Frame::Slice { descs, idx: 0 });

        Ok(())
    }

    fn parse_usize<'a, 'b, 'c, K: BUFRKey>(
        &self,
        state: &State,
        cache: &mut Cache<'c>,
        des: &'a K,
        data: &mut BitInput<'b>,
    ) -> Result<usize> {
        match des.f() {
            0 => {
                if let Some(e) = cache.get_b(des) {
                    let value = self.evalute(state, data, &e)?;

                    if let Some(v) = value.as_f64() {
                        Ok(v.floor() as usize)
                    } else {
                        Err(Error::ParseError(format!("Format Error")))
                    }
                } else {
                    Err(Error::ParseError(format!(
                        "Descriptor {:?} not found in Table B",
                        des
                    )))
                }
            }
            _ => Err(Error::ParseError(format!(
                "Descriptor {:?} not found in Table B",
                des
            ))),
        }
    }

    #[inline(always)]
    fn evalute<'a>(
        &self,
        state: &State,
        data: &mut BitInput<'a>,
        e: &ArchivedBTableEntry,
    ) -> Result<Value> {
        match e.bufr_unit.as_str() {
            "CCITT IA5" => {
                let total_bytes = state
                    .common_str_width
                    .unwrap_or(((e.bufr_datawidth_bits.to_native() as usize) + 7) / 8);
                let s = data.take_string(total_bytes as usize)?;
                return Ok(Value::String(s));
            }
            _ => {
                let datawidth = state.datawidth(e);
                let scale = state.scale(e) as f64;
                let reference_value = state.reference_value(e) as f64;
                let value = data.get_arbitary_bits(datawidth as usize)?;
                let mv = (1 << datawidth) - 1;
                if value == mv && e.fxy.x != 31 {
                    return Ok(Value::Missing);
                }
                let result = ((value as f64) + reference_value) * 10.0f64.powi(-scale as i32);
                return Ok(Value::Number(result));
            }
        }
    }

    fn try_compile_array_layout<'a, K: BUFRKey>(
        &self,
        body: &[K],
        repeat_count: usize,
        cache: &mut Cache<'a>,
    ) -> Result<Option<CompiledLayout<'a>>> {
        // Early rejection: too small
        if repeat_count < 16 {
            return Ok(None);
        }

        let mut compiler_state = CompilerState {
            common_scale: None,
            common_ref_value: None,
            common_data_width: None,
            temp_operator: None,
            common_str_width: None,
            local_data_width: None,
        };

        let mut fields = Vec::with_capacity(body.len());
        let mut total_bits = 0usize;

        for desc in body {
            match desc.f() {
                0 => {
                    // Element descriptor - compile field spec
                    let entry = cache.get_b(desc).ok_or_else(|| {
                        Error::ParseError(format!("Missing Table B entry for {:?}", desc))
                    })?;

                    // Reject strings
                    if entry.bufr_unit.as_str() == "CCITT IA5" {
                        return Ok(None);
                    }

                    // Compute effective parameters
                    let width = self.compute_effective_width(&compiler_state, entry);
                    let scale = self.compute_effective_scale(&compiler_state, entry);
                    let reference = self.compute_effective_reference(&compiler_state, entry);
                    let missing = if width == 64 {
                        u64::MAX
                    } else {
                        (1u64 << width) - 1
                    };

                    fields.push(FieldSpec {
                        fxy: FXY::new(desc.f(), desc.x(), desc.y()),
                        name: entry.element_name_en.as_str(),
                        unit: entry.bufr_unit.as_str(),
                        width_bits: width,
                        scale,
                        reference,
                        missing_value: missing,
                    });

                    total_bits += width as usize;

                    // Clear one-time operators after use
                    // 2-07 and 2-06 apply only to the next element
                    compiler_state.temp_operator = None;
                    compiler_state.local_data_width = None;
                }

                2 => {
                    if !self.apply_operator_to_compiler(&mut compiler_state, desc)? {
                        return Ok(None);
                    }
                }

                1 | 3 => {
                    // Nested replication or sequence - reject
                    return Ok(None);
                }

                _ => {
                    return Err(Error::ParseError(format!("Invalid F value: {}", desc.f())));
                }
            }
        }

        if compiler_state.temp_operator.is_some() {
            return Ok(None);
        }

        Ok(Some(CompiledLayout {
            fields,
            bits_per_element: total_bits,
        }))
    }

    fn apply_operator_to_compiler<K: BUFRKey>(
        &self,
        state: &mut CompilerState,
        operator: &K,
    ) -> Result<bool> {
        let x = operator.x();
        let y = operator.y() as i32;

        match x {
            1 => {
                // 2-01-YYY: data width change
                state.common_data_width = if y == 0 { None } else { Some(y) };
                Ok(true)
            }
            2 => {
                // 2-02-YYY: scale change
                state.common_scale = if y == 0 { None } else { Some(y) };
                Ok(true)
            }
            3 => {
                // 2-03-YYY: reference value change
                state.common_ref_value = if y == 0 { None } else { Some(y) };
                Ok(true)
            }
            5 => {
                // 2-05-YYY: string literal - consumes bits, reject
                Ok(false)
            }
            6 => {
                // 2-06-YYY: localized data width - affects only next element
                state.local_data_width = Some(y);
                Ok(true)
            }
            7 => {
                // 2-07-YYY: increase scale/width/ref - affects only next element
                state.temp_operator = Some(y);
                Ok(true)
            }
            8 => {
                // 2-08-YYY: character width - reject (affects strings)
                Ok(false)
            }
            _ => {
                // Unknown/unsupported operator - allow but ignore
                Ok(true)
            }
        }
    }

    #[inline]
    fn compute_effective_width(&self, state: &CompilerState, e: &ArchivedBTableEntry) -> u32 {
        if let Some(local_width) = state.local_data_width {
            return local_width as u32;
        }

        let unit = e.bufr_unit.as_str();
        let is_flag_or_code = matches!(
            unit,
            "flag table" | "flag-table" | "code table" | "code-table"
        );
        let delay_repeat_count = e.fxy.f.to_native() == 0 && e.fxy.x.to_native() == 31;
        let no_change = is_flag_or_code || delay_repeat_count;

        let base_width = if no_change {
            e.bufr_datawidth_bits.to_native()
        } else {
            state
                .common_data_width
                .map(|c| {
                    let (v, _) = e
                        .bufr_datawidth_bits
                        .to_native()
                        .overflowing_add_signed(c - 128);
                    v
                })
                .unwrap_or(e.bufr_datawidth_bits.to_native())
        };

        // 2-07-YYY: increase width by 10*Y bits
        if let Some(op) = state.temp_operator {
            base_width + (10 * op) as u32
        } else {
            base_width
        }
    }

    #[inline]
    fn compute_effective_scale(&self, state: &CompilerState, e: &ArchivedBTableEntry) -> i32 {
        let unit = e.bufr_unit.as_str();
        let is_flag_or_code = matches!(
            unit,
            "flag table" | "flag-table" | "code table" | "code-table"
        );
        let delay_repeat_count = e.fxy.f.to_native() == 0 && e.fxy.x.to_native() == 31;
        let no_change = is_flag_or_code || delay_repeat_count;

        let base_scale = if no_change {
            e.bufr_scale.to_native()
        } else {
            state
                .common_scale
                .map(|c| {
                    let (v, _) = e.bufr_scale.to_native().overflowing_add(128 - c);
                    v
                })
                .unwrap_or(e.bufr_scale.to_native())
        };

        if let Some(op) = state.temp_operator {
            base_scale + op
        } else {
            base_scale
        }
    }

    #[inline]
    fn compute_effective_reference(&self, state: &CompilerState, e: &ArchivedBTableEntry) -> i32 {
        let base_ref = e.bufr_reference_value.to_native();

        if let Some(op) = state.temp_operator {
            (base_ref as f32 * 10_f32.powi(op)) as i32
        } else {
            base_ref
        }
    }

    /// Fast path: decode array using pre-compiled layout
    fn parse_compiled_array<'a>(
        &self,
        layout: &CompiledLayout<'a>,
        repeat_count: usize,
        data: &mut BitInput,
        values: &mut BUFRParsed<'a>,
    ) -> Result<()> {
        let mut total_values = vec![vec![]; layout.fields.len()];
        // For each repetition
        for _ in 0..repeat_count {
            // For each field in the layout
            for (i, field_spec) in layout.fields.iter().enumerate() {
                let raw_value = data.get_arbitary_bits(field_spec.width_bits as usize)?;

                // Check for missing value (skip 0-31-YYY delayed replication counts)
                let value = if raw_value == field_spec.missing_value
                    && !(field_spec.fxy.f == 0 && field_spec.fxy.x == 31)
                {
                    MISS_VAL
                } else {
                    // Apply scale and reference
                    let scaled = ((raw_value as f64) + (field_spec.reference as f64))
                        * 10.0f64.powi(-field_spec.scale);
                    scaled
                };

                total_values[i].push(value);
            }
        }

        for (v, field) in total_values.into_iter().zip(layout.fields.iter()) {
            let mut array = values.start_array(0);
            array.set_values(v);
            array.finish(Some(field.name), Some(field.unit));
        }

        Ok(())
    }

    fn deal_with_operator<'s, 'a, C: Container<'s>, K: BUFRKey>(
        &self,
        state: &mut State,
        values: &mut C,
        operator: &K,
        data: &mut BitInput<'a>,
    ) -> Result<()> {
        let x = operator.x();
        let y = operator.y();

        match x {
            1 => match y {
                0 => {
                    state.common_data_width = None;
                }
                _ => {
                    state.common_data_width = Some(y);
                }
            },
            2 => match y {
                0 => {
                    state.common_scale = None;
                }
                _ => {
                    state.common_scale = Some(y);
                }
            },
            3 => match y {
                0 => {
                    state.common_ref_value = None;
                }
                _ => {
                    state.common_ref_value = Some(y);
                }
            },
            5 => {
                let string = data.take_string(y as usize)?;
                values.push(Value::String(string), "", "CAITT IA5");
            }

            6 => {
                let localized_width = y;
                state.local_data_width = Some(localized_width);
            }
            7 => {
                state.temp_operator = Some(y);
            }
            8 => match y {
                0 => {
                    state.common_str_width = None;
                }
                _ => {
                    state.common_str_width = Some(y as usize);
                }
            },
            _ => {}
        }

        Ok(())
    }

    #[cfg(feature = "opera")]
    fn parse_opera_bitmap<K: BUFRKey>(&self, des: &K) -> Option<&ArchivedBitMapEntry> {
        self.opera_bitmap_table
            .as_ref()
            .map(|t| t.lookup(des))
            .flatten()
    }

    // #[cfg(feature = "opera")]
    // fn parse_opera_array<'a>(
    //     &mut self,
    //     dw: u8,
    //     mut descs: VecDeque<FXY>,
    //     mut data: BitInput<'a>,
    // ) -> Result<(VecDeque<FXY>, BitInput<'a>)> {
    //     use crate::opera::OperaBitmapParser;

    //     let mut opera_bitmap = OperaBitmapParser::new(dw);

    //     while !descs.is_empty() {
    //         let (_descs, _data) = self.parser_inner(opera_bitmap.values(), descs, data)?;
    //         descs = _descs;
    //         data = _data;
    //     }
    //     Ok((descs, data))
    // }

    // fn seq_parser(descriptors: &[genlib::FXY]) -> Result<()> {}
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum Value {
    Number(f64),
    Missing,
    String(String),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Number(v) => write!(f, "{}", v),
            Value::String(v) => write!(f, "{}", v),
            Value::Missing => write!(f, "MISSING"),
        }
    }
}

impl Value {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Number(v) => Some(*v),
            Value::Missing => Some(MISS_VAL),
            Value::String(_) => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(v) => Some(v),
            Value::Number(_) => None,
            Value::Missing => None,
        }
    }

    pub fn as_bytes(&self) -> Option<Vec<u8>> {
        match self {
            Value::String(_) => None,
            Value::Number(n) => Some(n.to_le_bytes().to_vec()),
            Value::Missing => None,
        }
    }

    pub fn is_missing(&self) -> bool {
        matches!(self, Value::Missing)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BitInput<'a>(&'a [u8], usize);

impl<'a> BitInput<'a> {
    pub fn new(input: &[u8]) -> BitInput {
        BitInput(input, 0)
    }

    pub fn pointer(&self) -> usize {
        self.1
    }

    #[inline]
    pub fn take_string(&mut self, nbytes: usize) -> Result<String> {
        if nbytes == 0 {
            return Ok(String::new());
        }

        // Fast path: byte-aligned string reads
        if self.1 == 0 {
            if self.0.len() < nbytes {
                return Err(Error::ParseError("Not enough data for string".to_string()));
            }
            let s = String::from_utf8(self.0[..nbytes].to_vec())
                .map_err(|_| Error::ParseError("Invalid UTF-8 string".to_string()))?;
            self.0 = &self.0[nbytes..];
            self.1 = 0;
            return Ok(s);
        }

        // Slow path: unaligned reads
        let mut chars = Vec::with_capacity(nbytes);
        // let mut remaining_input = self;

        for _ in 0..nbytes {
            let byte_value = self.get_arbitary_bits(8)?;
            chars.push(byte_value as u8);
        }

        let s = String::from_utf8(chars)
            .map_err(|_| Error::ParseError("Invalid UTF-8 string".to_string()))?;
        Ok(s)
    }

    #[inline]
    pub fn get_arbitary_bits(&mut self, nbits: usize) -> Result<u64> {
        if nbits == 0 {
            return Ok(0);
        }

        // Fast path: byte-aligned reads for common bit widths
        if self.1 == 0 {
            return self.get_arbitary_bits_aligned(nbits);
        }

        // General path for unaligned reads
        self.get_arbitary_bits_unaligned(nbits)
    }

    /// Batch read multiple values with the same bit width
    /// Optimized for arrays of numeric data
    #[inline]
    pub fn get_batch_same_width(&mut self, nbits: usize, count: usize) -> Result<Vec<u64>> {
        if count == 0 {
            return Ok(Vec::new());
        }

        let mut result = Vec::with_capacity(count);

        // Fast path: byte-aligned and byte-multiple bit widths
        if self.1 == 0 && nbits % 8 == 0 {
            let bytes_per_item = nbits / 8;
            let total_bytes = bytes_per_item * count;

            if self.0.len() < total_bytes {
                return Err(Error::ParseError(
                    "Not enough data for batch read".to_string(),
                ));
            }

            match nbits {
                8 => {
                    // Optimized path for 8-bit values
                    for i in 0..count {
                        result.push(self.0[i] as u64);
                    }
                    self.0 = &self.0[count..];
                }
                16 => {
                    // Optimized path for 16-bit values
                    for i in 0..count {
                        let offset = i * 2;
                        let value = u16::from_be_bytes([self.0[offset], self.0[offset + 1]]) as u64;
                        result.push(value);
                    }
                    self.0 = &self.0[total_bytes..];
                }
                24 => {
                    // Optimized path for 24-bit values
                    for i in 0..count {
                        let offset = i * 3;
                        let value = ((self.0[offset] as u64) << 16)
                            | ((self.0[offset + 1] as u64) << 8)
                            | (self.0[offset + 2] as u64);
                        result.push(value);
                    }
                    self.0 = &self.0[total_bytes..];
                }
                32 => {
                    // Optimized path for 32-bit values
                    for i in 0..count {
                        let offset = i * 4;
                        let value = u32::from_be_bytes([
                            self.0[offset],
                            self.0[offset + 1],
                            self.0[offset + 2],
                            self.0[offset + 3],
                        ]) as u64;
                        result.push(value);
                    }
                    self.0 = &self.0[total_bytes..];
                }
                _ => {
                    // Generic byte-aligned path
                    for i in 0..count {
                        let offset = i * bytes_per_item;
                        let mut value: u64 = 0;
                        for j in 0..bytes_per_item {
                            value = (value << 8) | (self.0[offset + j] as u64);
                        }
                        result.push(value);
                    }
                    self.0 = &self.0[total_bytes..];
                }
            }

            return Ok(result);
        }

        // Non-aligned or non-byte-multiple: fall back to individual reads
        for _ in 0..count {
            result.push(self.get_arbitary_bits(nbits)?);
        }

        Ok(result)
    }

    /// Fast path for byte-aligned bit reads
    #[inline]
    fn get_arbitary_bits_aligned(&mut self, nbits: usize) -> Result<u64> {
        let byte_data = self.0;

        // Optimized paths for common bit widths
        match nbits {
            8 => {
                if byte_data.is_empty() {
                    return Err(Error::ParseError("Not enough data".to_string()));
                }
                self.0 = &self.0[1..];
                self.1 = 0;
                Ok(byte_data[0] as u64)
            }
            16 => {
                if byte_data.len() < 2 {
                    return Err(Error::ParseError("Not enough data".to_string()));
                }
                let value = u16::from_be_bytes([byte_data[0], byte_data[1]]) as u64;
                self.0 = &self.0[2..];
                self.1 = 0;
                Ok(value)
            }
            24 => {
                if byte_data.len() < 3 {
                    return Err(Error::ParseError("Not enough data".to_string()));
                }
                let value = ((byte_data[0] as u64) << 16)
                    | ((byte_data[1] as u64) << 8)
                    | (byte_data[2] as u64);
                self.0 = &self.0[3..];
                self.1 = 0;
                Ok(value)
            }
            32 => {
                if byte_data.len() < 4 {
                    return Err(Error::ParseError("Not enough data".to_string()));
                }
                let value =
                    u32::from_be_bytes([byte_data[0], byte_data[1], byte_data[2], byte_data[3]])
                        as u64;
                self.0 = &self.0[4..];
                self.1 = 0;
                Ok(value)
            }
            _ => {
                // Generic byte-aligned path
                let nbytes = (nbits + 7) / 8;
                if byte_data.len() < nbytes {
                    return Err(Error::ParseError("Not enough data".to_string()));
                }

                let mut value: u64 = 0;
                let full_bytes = nbits / 8;

                // Read full bytes
                for i in 0..full_bytes {
                    value = (value << 8) | (byte_data[i] as u64);
                }

                let remaining_bits = nbits % 8;
                if remaining_bits > 0 {
                    // Read partial byte
                    let last_byte = byte_data[full_bytes];
                    let shift = 8 - remaining_bits;
                    let mask = ((1u16 << remaining_bits) - 1) as u8;
                    let bits = (last_byte >> shift) & mask;
                    value = (value << remaining_bits) | (bits as u64);
                    self.0 = &self.0[full_bytes..];
                    self.1 = remaining_bits;
                    Ok(value)
                } else {
                    self.0 = &self.0[full_bytes..];
                    self.1 = 0;
                    Ok(value)
                }
            }
        }
    }

    /// Optimized path for unaligned bit reads
    /// Reads up to 64 bits from an unaligned position in one go
    #[inline]
    fn get_arbitary_bits_unaligned(&mut self, nbits: usize) -> Result<u64> {
        if nbits > 64 {
            return Err(Error::ParseError(
                "Cannot read more than 64 bits".to_string(),
            ));
        }

        let bit_offset = self.1;

        // Calculate how many bytes we need to read
        // We need enough bytes to cover: bit_offset + nbits
        let total_bits_needed = bit_offset + nbits;
        let bytes_needed = (total_bits_needed + 7) / 8;

        if self.0.len() < bytes_needed {
            return Err(Error::ParseError("Not enough data".to_string()));
        }

        // Read up to 8 bytes into a u64 buffer for fast bit extraction
        let mut buffer: u64 = 0;
        let bytes_to_read = bytes_needed.min(8);

        for i in 0..bytes_to_read {
            buffer = (buffer << 8) | (self.0[i] as u64);
        }

        // If we need more than 8 bytes, handle the extra byte
        if bytes_needed > 8 {
            // This is rare - only happens for very unaligned 64-bit reads
            // Shift what we have and add the 9th byte
            let ninth_byte = self.0[8] as u64;
            let bits_from_ninth = total_bits_needed - 64;
            buffer = (buffer << bits_from_ninth) | (ninth_byte >> (8 - bits_from_ninth));
        }

        // Extract the desired bits
        // The bits we want are in the high portion of the buffer
        let bits_in_buffer = bytes_to_read * 8;
        let shift = bits_in_buffer - bit_offset - nbits;
        let mask = if nbits == 64 {
            u64::MAX
        } else {
            (1u64 << nbits) - 1
        };
        let value = (buffer >> shift) & mask;

        // Update state
        let new_bit_position = self.1 + nbits;
        let bytes_consumed = new_bit_position / 8;
        self.0 = &self.0[bytes_consumed..];
        self.1 = new_bit_position % 8;

        Ok(value)
    }
}

trait Container<'a>
where
    Self: Sized,
{
    fn push(&mut self, value: Value, name: &'a str, unit: &'a str);

    fn start_repeating<'b>(&'b mut self, time: usize) -> Repeating<'a, 'b>;
}

impl<'a> Container<'a> for BUFRParsed<'a> {
    fn push(&mut self, value: Value, name: &'a str, unit: &'a str) {
        self.push(value, name, unit);
    }

    fn start_repeating<'s>(&'s mut self, time: usize) -> Repeating<'a, 's> {
        self.start_repeating(time)
    }
}

impl<'a, 'b> Container<'a> for Repeating<'a, 'b> {
    fn push(&mut self, value: Value, _name: &'a str, _unit: &'a str) {
        self.push(value);
    }

    fn start_repeating<'s>(&'s mut self, time: usize) -> Repeating<'a, 's> {
        Repeating {
            parsed: self.parsed,
            values: Vec::with_capacity(time),
        }
    }
}

#[derive(Clone)]
pub struct BUFRParsed<'a> {
    records: Vec<BUFRRecord<'a>>,
}

impl<'a> BUFRParsed<'a> {
    pub fn new() -> Self {
        Self { records: vec![] }
    }

    fn push(&mut self, value: Value, element_name: &'a str, unit: &'a str) {
        self.records.push(BUFRRecord {
            name: Some(Cow::Borrowed(element_name)),
            values: BUFRData::Single(value),
            unit: Some(Cow::Borrowed(unit)),
        });
    }

    fn start_repeating<'s>(&'s mut self, time: usize) -> Repeating<'a, 's> {
        Repeating {
            parsed: self,
            values: Vec::with_capacity(time),
        }
    }

    fn start_array<'s>(&'s mut self, time: usize) -> Array<'a, 's> {
        Array {
            parsed: self,
            values: Vec::with_capacity(time),
        }
    }

    pub fn into_owned(&self) -> BUFRParsed<'static> {
        BUFRParsed {
            records: self.records.iter().map(|r| r.into_owned()).collect(),
        }
    }
}

struct Array<'a, 's> {
    parsed: &'s mut BUFRParsed<'a>,
    values: Vec<f64>,
}

impl<'a> Array<'a, '_> {
    fn set_values(&mut self, values: Vec<f64>) {
        self.values = values;
    }

    fn push(&mut self, v: f64) {
        self.values.push(v);
    }

    fn finish(self, name: Option<&'a str>, unit: Option<&'a str>) {
        let recording = BUFRRecord {
            name: name.map(|n| Cow::Borrowed(n)),
            values: BUFRData::Array(self.values),
            unit: unit.map(|u| Cow::Borrowed(u)),
        };
        self.parsed.records.push(recording);
    }
}

struct Repeating<'a, 's> {
    parsed: &'s mut BUFRParsed<'a>,
    values: Vec<Value>,
}

impl<'a, 's> Repeating<'a, 's> {
    fn push(&mut self, value: Value) {
        self.values.push(value);
    }

    fn finish(self) {
        let recording = BUFRRecord {
            name: None,
            values: BUFRData::Repeat(self.values),
            unit: None,
        };
        self.parsed.records.push(recording);
    }
}

#[derive(Debug, Clone)]
pub enum BUFRData {
    Repeat(Vec<Value>),
    Single(Value),
    Array(Vec<f64>),
}

#[derive(Clone)]
pub struct BUFRRecord<'a> {
    // pub name: Option<&'a str>,
    pub name: Option<Cow<'a, str>>,
    pub values: BUFRData,
    pub unit: Option<Cow<'a, str>>,
}

impl BUFRRecord<'_> {
    pub fn into_owned(&self) -> BUFRRecord<'static> {
        BUFRRecord {
            name: self.name.as_ref().map(|s| Cow::Owned(s.to_string())),
            values: match &self.values {
                BUFRData::Single(v) => BUFRData::Single(v.clone()),
                BUFRData::Repeat(vs) => BUFRData::Repeat(vs.clone()),
                BUFRData::Array(a) => BUFRData::Array(a.clone()),
            },
            unit: self.unit.as_ref().map(|s| Cow::Owned(s.to_string())),
        }
    }
}

impl Display for BUFRRecord<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let is_print_unit = match self.unit.as_ref().map(|s| &**s) {
            Some("CAITT IA5" | "code table" | "code-table" | "flag table" | "flag-table") => false,
            None => false,
            _ => true,
        };

        if self.name.is_none() {
            return Ok(());
        }

        let name = self.name.as_ref().unwrap();
        let width = f.width().unwrap_or(0);

        match &self.values {
            BUFRData::Single(v) => {
                if width > 0 {
                    write!(f, "{:<width$} : ", name, width = width)?;
                } else {
                    write!(f, "{} : ", name)?;
                }

                match v {
                    Value::Missing => write!(f, "MISSING")?,
                    Value::String(s) => write!(f, "\"{}\"", s)?,
                    Value::Number(n) => {
                        if is_print_unit {
                            write!(f, "{:>12.6} {}", n, self.unit.as_ref().unwrap())?;
                        } else {
                            write!(f, "{}", n)?;
                        }
                    }
                }
            }
            BUFRData::Repeat(vs) => {
                self.format_sequence(f, name, vs, is_print_unit, width)?;
            }
            BUFRData::Array(a) => {
                self.format_array(f, name, a, is_print_unit, width)?;
            }
        }

        Ok(())
    }
}

impl BUFRRecord<'_> {
    fn format_sequence(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        name: &str,
        values: &[Value],
        is_print_unit: bool,
        width: usize,
    ) -> std::fmt::Result {
        let missing_count = values.iter().filter(|v| v.is_missing()).count();

        if width > 0 {
            write!(f, "{:<width$} : ", name, width = width)?;
        } else {
            write!(f, "{} : ", name)?;
        }

        write!(f, "[len={}", values.len())?;
        if missing_count > 0 {
            write!(f, ", missing={}", missing_count)?;
        }
        write!(f, "] ")?;

        if values.is_empty() {
            write!(f, "[]")?;
            return Ok(());
        }

        let show_limit = 6;
        if values.len() <= show_limit {
            write!(f, "[")?;
            for (i, v) in values.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                self.format_value(f, v, is_print_unit)?;
            }
            write!(f, "]")?;
        } else {
            write!(f, "[")?;
            for (i, v) in values.iter().take(3).enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                self.format_value(f, v, is_print_unit)?;
            }
            write!(f, " ... ")?;
            for (i, v) in values.iter().skip(values.len() - 2).enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                self.format_value(f, v, is_print_unit)?;
            }
            write!(f, "]")?;
        }

        Ok(())
    }

    fn format_array(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        name: &str,
        values: &[f64],
        is_print_unit: bool,
        width: usize,
    ) -> std::fmt::Result {
        let missing_count = values.iter().filter(|&&v| v == MISS_VAL).count();
        let valid_values: Vec<f64> = values.iter().copied().filter(|&v| v != MISS_VAL).collect();

        if width > 0 {
            write!(f, "{:<width$} : ", name, width = width)?;
        } else {
            write!(f, "{} : ", name)?;
        }

        write!(f, "[len={}", values.len())?;
        if missing_count > 0 {
            write!(f, ", missing={}", missing_count)?;
        }

        // 显示统计信息
        if !valid_values.is_empty() {
            let min = valid_values.iter().copied().fold(f64::INFINITY, f64::min);
            let max = valid_values
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max);
            let mean = valid_values.iter().sum::<f64>() / valid_values.len() as f64;

            write!(f, ", min={:.3}, max={:.3}, mean={:.3}", min, max, mean)?;
        }
        write!(f, "]")?;

        if is_print_unit {
            if let Some(unit) = &self.unit {
                write!(f, " {}", unit)?;
            }
        }

        // 显示样例值
        if !values.is_empty() {
            let show_limit = 6;
            if values.len() <= show_limit {
                write!(f, "\n  [")?;
                for (i, v) in values.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    if *v == MISS_VAL {
                        write!(f, "MISSING")?;
                    } else {
                        write!(f, "{:.3}", v)?;
                    }
                }
                write!(f, "]")?;
            } else {
                write!(f, "\n  [")?;
                for (i, v) in values.iter().take(3).enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    if *v == MISS_VAL {
                        write!(f, "MISSING")?;
                    } else {
                        write!(f, "{:.3}", v)?;
                    }
                }
                write!(f, " ... ")?;
                for (i, v) in values.iter().skip(values.len() - 2).enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    if *v == MISS_VAL {
                        write!(f, "MISSING")?;
                    } else {
                        write!(f, "{:.3}", v)?;
                    }
                }
                write!(f, "]")?;
            }
        }

        Ok(())
    }

    fn format_value(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        value: &Value,
        is_print_unit: bool,
    ) -> std::fmt::Result {
        match value {
            Value::Missing => write!(f, "MISSING"),
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Number(n) => {
                if is_print_unit {
                    write!(f, "{:.3}", n)
                } else {
                    write!(f, "{}", n)
                }
            }
        }
    }
}

impl Display for BUFRParsed<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "BUFR Parsed Data ({} records)", self.records.len())?;

        // 计算最长的名称长度用于对齐
        let max_name_len = self
            .records
            .iter()
            .filter_map(|r| r.name.as_ref())
            .map(|n| n.len())
            .max()
            .unwrap_or(0)
            .min(50); // 限制最大宽度

        for record in &self.records {
            writeln!(f, "{:<max_name_len$}", record, max_name_len = max_name_len)?;
        }

        Ok(())
    }
}

impl BUFRParsed<'_> {
    pub fn record_count(&self) -> usize {
        self.records.len()
    }

    pub fn records(&self) -> &[BUFRRecord<'_>] {
        &self.records
    }

    pub fn display_compact(&self) -> CompactDisplay<'_> {
        CompactDisplay(self)
    }

    pub fn display_detailed(&self) -> DetailedDisplay<'_> {
        DetailedDisplay(self)
    }
}

pub struct CompactDisplay<'a>(&'a BUFRParsed<'a>);

impl Display for CompactDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for record in &self.0.records {
            writeln!(f, "{}", record)?;
        }
        Ok(())
    }
}

pub struct DetailedDisplay<'a>(&'a BUFRParsed<'a>);

impl Display for DetailedDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "BUFR Parsed Data - Detailed View")?;
        writeln!(f)?;

        let total_records = self.0.records.len();
        let single_count = self
            .0
            .records
            .iter()
            .filter(|r| matches!(r.values, BUFRData::Single(_)))
            .count();
        let array_count = self
            .0
            .records
            .iter()
            .filter(|r| matches!(r.values, BUFRData::Array(_)))
            .count();
        let repeat_count = self
            .0
            .records
            .iter()
            .filter(|r| matches!(r.values, BUFRData::Repeat(_)))
            .count();

        writeln!(f, "Statistics:")?;
        writeln!(f, "  Total records:     {}", total_records)?;
        writeln!(f, "  Single values:     {}", single_count)?;
        writeln!(f, "  Arrays:            {}", array_count)?;
        writeln!(f, "  Repeated values:   {}", repeat_count)?;
        writeln!(f)?;

        let max_name_len = self
            .0
            .records
            .iter()
            .filter_map(|r| r.name.as_ref())
            .map(|n| n.len())
            .max()
            .unwrap_or(0)
            .min(50);

        for (idx, record) in self.0.records.iter().enumerate() {
            writeln!(
                f,
                "Record {}: {:<max_name_len$}",
                idx + 1,
                record,
                max_name_len = max_name_len
            )?;
        }

        Ok(())
    }
}

enum Frame<'v, 'a> {
    Slice {
        descs: Descs<'v>,
        idx: usize,
    },
    Repeat {
        descs: Descs<'v>,
        times: usize,
        current: usize,
    },
    CompiledArray {
        layout: CompiledLayout<'a>,
        times: usize,
    },
}

#[derive(Clone, Copy)]
enum Descs<'v> {
    Raw(&'v [genlib::FXY]),
    Archived(&'v [ArchivedFXY]),
}

impl Descs<'_> {
    fn len(&self) -> usize {
        match self {
            Descs::Raw(d) => d.len(),
            Descs::Archived(d) => d.len(),
        }
    }

    fn total_bits(&self, state: &State, cache: &mut Cache) -> Result<usize> {
        match self {
            Descs::Raw(d) => self._total_bits(state, cache, d),
            Descs::Archived(d) => self._total_bits(state, cache, d),
        }
    }

    fn _total_bits<K: BUFRKey>(
        &self,
        state: &State,
        cache: &mut Cache,
        decs: &[K],
    ) -> Result<usize> {
        let mut total_width = 0;
        for des in decs {
            let e = cache.get_b(des).ok_or(Error::TableNotFoundEmpty)?;
            let width = state.datawidth(e);
            total_width += width as usize;
        }

        Ok(total_width)
    }
}

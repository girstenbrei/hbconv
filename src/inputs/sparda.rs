use std::{io::Read, iter::Skip};

use chrono::NaiveDate;
use csv::{DeserializeRecordsIntoIter, ReaderBuilder};
use rusty_money::{iso::{Currency, EUR}, Money};
use encoding_rs::WINDOWS_1252;
use encoding_rs_io::{DecodeReaderBytes, DecodeReaderBytesBuilder};
use miette::{Context, IntoDiagnostic, Report};
use serde::Deserialize;

use crate::{
    homebank::{Payment, Record},
    RecordIteratorRes,
};

struct Sparda {
    buchungstag: NaiveDate,
    _wertstellungstag: NaiveDate,
    gegeniban: String,
    name_gegenkonto: String,
    verwendungszweck: String,
    umsatz: Money<'static, Currency>,
    _währung: String,
}

#[derive(Debug, Deserialize)]
struct SpardaIR {
    buchungstag: String,
    wertstellungstag: String,
    gegeniban: String,
    name_gegenkonto: String,
    verwendungszweck: String,
    umsatz: String,
    währung: String,
}

impl TryFrom<SpardaIR> for Sparda {
    type Error = Report;

    fn try_from(value: SpardaIR) -> Result<Self, Self::Error> {

        Ok(Self {
            buchungstag: NaiveDate::parse_from_str(&value.buchungstag, "%Y-%m-%d")
                .into_diagnostic()
                .wrap_err("Failed converting buchungstag into datetime")?,
            _wertstellungstag: NaiveDate::parse_from_str(&value.wertstellungstag, "%Y-%m-%d")
                .into_diagnostic()
                .wrap_err("Failed converting buchungstag into datetime")?,
            gegeniban: value.gegeniban,
            name_gegenkonto: value.name_gegenkonto,
            verwendungszweck: value.verwendungszweck,
            umsatz: Money::from_str(
                value.umsatz.trim_matches('"'), EUR)
                .into_diagnostic()
                .wrap_err("Failed converting currency")?,
            _währung: value.währung,
        })
    }
}

impl From<Sparda> for Record {
    fn from(val: Sparda) -> Self {
        Self {
            date: val.buchungstag,
            payment: Payment::ElectronicPayment,
            info: val.gegeniban,
            payee: val.name_gegenkonto,
            memo: val.verwendungszweck,
            amount: val.umsatz,
            category: String::new(),
            tags: Vec::new(),
        }
    }
}

pub struct TeoIter<R: Read> {
    deser: Skip<DeserializeRecordsIntoIter<DecodeReaderBytes<R, Vec<u8>>, SpardaIR>>,
}

impl<R: Read> TeoIter<R> {
    pub fn new(rdr: R) -> Self {
        // Sparda does not encode their csvs as UTF8...
        let decoder = DecodeReaderBytesBuilder::new()
            .encoding(Some(WINDOWS_1252))
            .build(rdr);

        let rdr = ReaderBuilder::new()
            .delimiter(b';')
            .has_headers(false)
            .quoting(false)
            .flexible(true)
            .from_reader(decoder);

        let deser: DeserializeRecordsIntoIter<DecodeReaderBytes<R, Vec<u8>>, SpardaIR> =
            rdr.into_deserialize();

        // We skip the first 10 lines outright, because apparently Sparda
        // has an insane idea about what constitutes a valid CSV file.
        let skip = deser.skip(10);

        Self { deser: skip }
    }
}

impl<R: Read> Iterator for TeoIter<R> {
    type Item = RecordIteratorRes;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self
            .deser
            .next()?
            .map(Sparda::try_from)
            .into_diagnostic()
            .wrap_err("Failed deserializing record");

        match next {
            Ok(Err(e)) => Some(Err(e)),
            Err(e) => Some(Err(e)),
            Ok(Ok(v)) => Some(Ok(v.into())),
        }
    }
}

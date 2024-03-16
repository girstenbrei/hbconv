use chrono::NaiveDate;
use csv::{DeserializeRecordsIntoIter, ReaderBuilder};
use currency_rs::{Currency, CurrencyOpts};
use miette::{Context, IntoDiagnostic, Report, Result, miette};
use serde::Deserialize;
use std::io::{BufRead, BufReader, Read};

use crate::homebank::{Payment, Record};

#[derive(Debug)]
pub struct Postbank {
    buchungstag: NaiveDate,
    _wert: NaiveDate,
    _umsatzart: String,
    auftraggeber: String,
    verwendungszweck: String,
    _iban: String,
    _bic: String,
    kundenreferenz: String,
    _mandatsreferenz: String,
    _gläubiger_id: String,
    _fremde_gebühren: String,
    betrag: Currency,
    _abweichender_empfänger: String,
    _count_aufträge: String,
    _count_schecks: String,
    _soll: String,
    _haben: String,
    _währung: Currency,
}

#[derive(Debug, Deserialize)]
struct PostbankIR {
    buchungstag: String,
    wert: String,
    _umsatzart: String,
    auftraggeber: String,
    verwendungszweck: String,
    _iban: String,
    _bic: String,
    kundenreferenz: String,
    _mandatsreferenz: String,
    _gläubiger_id: String,
    _fremde_gebühren: String,
    betrag: String,
    _abweichender_empfänger: String,
    _count_aufträge: String,
    _count_schecks: String,
    _soll: String,
    _haben: String,
    _währung: String,
}

pub struct PostbankIter<R: Read> {
    deser: DeserializeRecordsIntoIter<BufReader<R>, PostbankIR>,
}

impl<R: Read> PostbankIter<R> {
    pub fn new(rdr: R) -> Self {
        let mut buf_read = BufReader::new(rdr);
        let mut buf = Vec::new();
        // We skip the first 7 lines outright, because apparently Postbank
        // has an insane idea about what constitutes a valid CSV file.
        for _i in 0..=7 {
            buf_read.read_until(b'\n', &mut buf).expect("Bum");
        }
        // println!("Skipped|{}|", String::from_utf8_lossy(&buf));

        let rdr = ReaderBuilder::new()
            .delimiter(b';')
            .has_headers(false)
            .quoting(false)
            .flexible(true)
            .from_reader(buf_read);

        Self {
            deser: rdr.into_deserialize(),
        }
    }
}

impl<R: Read> Iterator for PostbankIter<R> {
    type Item = Result<Postbank>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self
            .deser
            .next()?
            .map(Postbank::try_from)
            .into_diagnostic()
            .wrap_err("Failed deserializing record");

        match next {
            Ok(v) => Some(v),
            Err(e) => Some(Err(e)),
        }
    }
}

impl TryFrom<PostbankIR> for Postbank {
    type Error = Report;
    fn try_from(value: PostbankIR) -> Result<Self> {
        let currency_opts = CurrencyOpts::new()
            .set_decimal(",")
            .set_separator("")
            .set_negative_pattern("-#");

        Ok(Self {
            buchungstag: NaiveDate::parse_from_str(&value.buchungstag, "%d.%m.%Y")
                .into_diagnostic()
                .wrap_err("Failed converting buchungstag into datetime")?,
            _wert: NaiveDate::parse_from_str(&value.wert, "%d.%m.%Y")
                .into_diagnostic()
                .wrap_err("Failed converting wert into datetime")?,
            _umsatzart: value._umsatzart,
            auftraggeber: value.auftraggeber,
            verwendungszweck: value.verwendungszweck,
            _iban: value._iban,
            _bic: value._bic,
            kundenreferenz: value.kundenreferenz,
            _mandatsreferenz: value._mandatsreferenz,
            _gläubiger_id: value._gläubiger_id,
            _fremde_gebühren: value._fremde_gebühren,
            betrag: Currency::new_string(&value.betrag, Some(currency_opts.clone()))
            .map_err(|e|miette!("{:?}", e))
            .wrap_err("Failed converting currency")?,
            _abweichender_empfänger: value._abweichender_empfänger,
            _count_aufträge: value._count_aufträge,
            _count_schecks: value._count_schecks,
            _soll: value._soll,
            _haben: value._haben,
            _währung: Currency::new_string(&value._währung, Some(currency_opts.clone()))
                .map_err(|e|miette!("{:?}", e))
                .wrap_err("Failed converting currency")?,
        })
    }
}

impl From<Postbank> for Record {
    fn from(val: Postbank) -> Self {
        Self {
            date: val.buchungstag,
            payment: Payment::ElectronicPayment,
            info: val.kundenreferenz,
            payee: val.auftraggeber,
            memo: val.verwendungszweck,
            amount: val.betrag,
            category: String::new(),
            tags: Vec::new(),
        }
    }
}

#[cfg(test)]
mod test {
    use miette::Result;

    use super::*;

    #[test]
    fn test_to_iter() {
        let input = b"\n\n\n\n\n\n\n\n7.3.2024;7.3.2024;SEPA Lastschrift;Woopsie;Doopsie;DE123;;ABCD;EFG;DE123;;-25,88;;;;-25,88;;EUR\n";

        let postbank_iter = PostbankIter::new(&input[..]);
        let element: Vec<Result<Postbank>> = postbank_iter.collect();

        assert_eq!(element.len(), 1);
        assert!(element[0].is_ok());
    }
}

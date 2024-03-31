//! Format deserializer for the Homebank csv format.
//!
//! Docs are taken from http://homebank.free.fr/help/misc-csvformat.html#txn .

use std::io;

use chrono::NaiveDate;
use csv::{Writer, WriterBuilder};
use miette::{Context, IntoDiagnostic, Result};
use rusty_money::{iso::Currency, Money};
use serde::{Deserialize, Serialize};

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Deserialize, Serialize)]
#[repr(u8)]
pub enum Payment {
    None = 0,
    CreditCard = 1,
    Check = 2,
    Cash = 3,
    // not allowed because CSV do not support multiple accounts => will be imported as 4 = bank transfer
    BankTransfer = 4,
    InternalTransfer = 5,
    DebitCard = 6,
    StandingOrder = 7,
    ElectronicPayment = 8,
    Deposit = 9,
    FinancialInstitutionFee = 10,
    DirectDebit = 11,
}

#[derive(Debug)]
pub struct Record {
    pub date: NaiveDate,
    pub payment: Payment,
    pub info: String,
    pub payee: String,
    pub memo: String,
    pub amount: Money<'static, Currency>,
    pub category: String,
    // tags separated by space
    pub tags: Vec<String>,
}

impl Record {
    pub fn writer<W: io::Write>(writer: W) -> Writer<W> {
        WriterBuilder::new()
            .delimiter(b';')
            .has_headers(false)
            .from_writer(writer)
    }

    pub fn write<W: io::Write>(self, writer: &mut Writer<W>) -> Result<()> {
        let ir: RecordIR = self.into();

        writer
            .serialize(ir)
            .into_diagnostic()
            .wrap_err("Failed serializing hb record to output file")
    }
}

#[derive(Debug, Serialize)]
struct RecordIR {
    date: String,
    payment: u8,
    info: String,
    payee: String,
    memo: String,
    amount: String,
    category: String,
    // tags separated by space
    tags: String,
}

impl From<Record> for RecordIR {
    fn from(value: Record) -> Self {
        Self {
            date: value.date.format("%Y-%m-%d").to_string(),
            payment: value.payment as u8,
            info: value.info,
            payee: value.payee,
            memo: value.memo,
            amount: value.amount.to_string(),
            category: value.category,
            tags: value.tags.join(" "),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;
    use rusty_money::iso::EUR;

    #[test]
    fn test_basic_deser() {
        let expected = b"2015-02-04;0;;;Some cash;-40,00;Bill:Withdrawal of cash;tag1 tag2\n2015-02-04;1;;;Internet DSL;-45,00;Inline service/Internet;tag2 my-tag3\n";

        let date =
            NaiveDate::parse_from_str("2015-02-04", "%Y-%m-%d").expect("Failed parsing date");

        let data = vec![
            Record {
                date,
                payment: Payment::None,
                info: "".to_string(),
                payee: "".to_string(),
                memo: "Some cash".to_string(),
                amount: Money::from_str("40,00", EUR).expect("Failed parsing money"),
                category: "Bill:Withdrawal of cash".to_string(),
                tags: vec!["tag1".to_string(), "tag2".to_string()],
            },
            Record {
                date,
                payment: Payment::CreditCard,
                info: "".to_string(),
                payee: "".to_string(),
                memo: "Internet DSL".to_string(),
                amount: Money::from_str("-45,00", EUR).expect("Failed parsing money"),
                category: "Inline service/Internet".to_string(),
                tags: vec!["tag2".to_string(), "my-tag3".to_string()],
            },
        ];

        let mut writer = Vec::new();
        let mut wtr = csv::WriterBuilder::new()
            .delimiter(b';')
            .has_headers(false)
            .from_writer(&mut writer);

        for record in data {
            let ir_record: RecordIR = record.into();

            wtr.serialize(ir_record).expect("Failed serializing record");
        }
        wtr.flush().expect("Failed flushing writer");
        drop(wtr);

        let writer = String::from_utf8_lossy(&writer);
        let expected = String::from_utf8_lossy(&expected[..]);

        assert_eq!(writer, expected);
    }
}

use std::fmt;

use crate::{items::*, parsers::z3::z3parser::Z3Parser};

////////////
// General
////////////

pub trait DisplayWithCtxt<Ctxt, Data: Copy>: Sized {
    fn fmt_with<'a>(&'a self, f: &mut fmt::Formatter<'_>, ctxt: &Ctxt, data: Data) -> fmt::Result;
    fn with<'a, 'b>(&'a self, ctxt: &'b Ctxt) -> DisplayWrapper<'a, 'b, Ctxt, Data, Self> where Data: Default {
        self.with_data(ctxt, Data::default())
    }
    fn with_data<'a, 'b>(&'a self, ctxt: &'b Ctxt, data: Data) -> DisplayWrapper<'a, 'b, Ctxt, Data, Self> {
        DisplayWrapper { inner: self, ctxt, data }
    }
}

pub struct DisplayWrapper<'a, 'b, Ctxt, Data: Copy, T: DisplayWithCtxt<Ctxt, Data>> {
    pub inner: &'a T,
    pub ctxt: &'b Ctxt,
    pub data: Data,
}

impl<'a, 'b, Ctxt, Data: Copy, T: DisplayWithCtxt<Ctxt, Data>> fmt::Display for DisplayWrapper<'a, 'b, Ctxt, Data, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt_with(f, self.ctxt, self.data)
    }
}

////////////
// Items
////////////

#[derive(Debug)]
pub struct DisplayCtxt<'a> {
    pub parser: &'a Z3Parser,

    pub display_term_ids: bool,
    pub display_quantifier_name: bool,
    pub use_mathematical_symbols: bool,
}

#[derive(Debug, Clone, Copy)]
struct DisplayData<'a> {
    children: &'a [TermIdx],
    quant: Option<&'a Quantifier>,
    bind_power: u8,
}
const NO_BIND: u8 = 0;
const QUANT_BIND: u8 = 3;
const INFIX_BIND: u8 = 15;
const PREFIX_BIND: u8 = 31;

////////////
// Item Idx defs
////////////

impl DisplayWithCtxt<DisplayCtxt<'_>, ()> for TermIdx {
    fn fmt_with(&self, f: &mut fmt::Formatter<'_>, ctxt: &DisplayCtxt<'_>, _data: ()) -> fmt::Result {
        let data = DisplayData {
            children: &Vec::new(),
            quant: None,
            bind_power: NO_BIND,
        };
        write!(f, "{}", ctxt.parser[*self].with_data(ctxt, data))
    }
}

impl DisplayWithCtxt<DisplayCtxt<'_>, ()> for QuantIdx {
    fn fmt_with(&self, f: &mut fmt::Formatter<'_>, ctxt: &DisplayCtxt<'_>, data: ()) -> fmt::Result {
        let quant = &ctxt.parser[*self];
        if let Some(term) = quant.term {
            term.fmt_with(f, ctxt, data)
        } else {
            let QuantKind::Other(name) = &quant.kind else {
                panic!()
            };
            write!(f, "{}", name)
        }
    }
}

////////////
// Item defs
////////////

impl DisplayWithCtxt<DisplayCtxt<'_>, DisplayData<'_>> for Term {
    fn fmt_with(&self, f: &mut fmt::Formatter<'_>, ctxt: &DisplayCtxt<'_>, data: DisplayData<'_>) -> fmt::Result {
        let data = DisplayData {
            children: &self.child_ids,
            ..data
        };

        if ctxt.display_term_ids {
            write!(f, "[{}]", self.id)?;
        }
        if let Some(meaning) = &self.meaning {
            write!(f, "{}", meaning.with_data(ctxt, data))
        } else {
            write!(f, "{}", self.kind.with_data(ctxt, data))
        }
    }
}

impl DisplayWithCtxt<DisplayCtxt<'_>, DisplayData<'_>> for TermKind {
    fn fmt_with(&self, f: &mut fmt::Formatter<'_>, ctxt: &DisplayCtxt<'_>, data: DisplayData<'_>) -> fmt::Result {
        match self {
            TermKind::Var(idx) => {
                let this = data.quant.map(|q| &q.vars).unwrap_or(&None);
                write!(f, "{}", VarNames::get_name(this, *idx))
            }
            TermKind::ProofOrApp(poa) =>
                write!(f, "{}", poa.with_data(ctxt, data)),
            TermKind::Quant(idx) =>
                write!(f, "{}", ctxt.parser[*idx].with_data(ctxt, data)),
        }
    }
}

enum ProofOrAppKind {
    Unary,
    Inline,
    Pattern,
    OtherApp,
    Proof,
}
impl DisplayWithCtxt<DisplayCtxt<'_>, DisplayData<'_>> for ProofOrApp {
    fn fmt_with(&self, f: &mut fmt::Formatter<'_>, ctxt: &DisplayCtxt<'_>, mut data: DisplayData<'_>) -> fmt::Result {
        let math = ctxt.use_mathematical_symbols;
        use ProofOrAppKind::*;
        let (name, kind) = match self.name.as_ref() {
            name if self.is_proof => (name, Proof),
            "not" => (if math { "¬" } else { "!" }, Unary),
            "-" if data.children.len() == 1 => ("-", Unary),

            "and" => (if math { "∧" } else { "&&" }, Inline),
            "or" => (if math { "∨" } else { "||" }, Inline),
            "<=" => (if math { "≤" } else { "<=" }, Inline),
            ">=" => (if math { "≥" } else { ">=" }, Inline),
            op@("=" | "+" | "-" | "*" | "/" | "<" | ">") => (op, Inline),

            "pattern" => ("pattern", Pattern),

            name => (name, OtherApp),
        };
        match kind {
            Unary => {
                assert!(data.bind_power <= PREFIX_BIND);
                data.bind_power = PREFIX_BIND;
                assert_eq!(data.children.len(), 1);
                let child = data.children[0];
                write!(f, "{name}{}", ctxt.parser[child].with_data(ctxt, data))
            }
            Inline => {
                let need_brackets = data.bind_power >= INFIX_BIND;
                data.bind_power = INFIX_BIND;

                if need_brackets {
                    write!(f, "(")?;
                }
                for (idx, child) in data.children.iter().enumerate() {
                    if idx != 0 {
                        write!(f, " {name} ")?;
                    }
                    write!(f, "{}", ctxt.parser[*child].with_data(ctxt, data))?;
                }
                if need_brackets {
                    write!(f, ")")?;
                }
                Ok(())
            }
            Pattern => {
                data.bind_power = NO_BIND;
                write!(f, "{{")?;
                for (idx, child) in data.children.iter().enumerate() {
                    if idx != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", ctxt.parser[*child].with_data(ctxt, data))?;
                }
                write!(f, "}}")
            }
            OtherApp | Proof => {
                write!(f, "{name}")?;
                if data.children.is_empty() {
                    return Ok(());
                }
                // BIND_POWER is PREFIX_BIND (highest)
                data.bind_power = NO_BIND;
                write!(f, "(")?;
                for (idx, child) in data.children.iter().enumerate() {
                    if idx != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", ctxt.parser[*child].with_data(ctxt, data))?;
                }
                write!(f, ")")
            }
        }
    }
}

impl DisplayWithCtxt<DisplayCtxt<'_>, DisplayData<'_>> for Meaning {
    fn fmt_with(&self, f: &mut fmt::Formatter<'_>, _ctxt: &DisplayCtxt<'_>, _data: DisplayData<'_>) -> fmt::Result {
        match self.theory.as_ref() {
            "arith" | "bv" => write!(f, "{}", self.value),
            theory=> write!(f, "/{theory} {}\\", self.value),
        }
    }
}

impl DisplayWithCtxt<DisplayCtxt<'_>, DisplayData<'_>> for Quantifier {
    fn fmt_with(&self, f: &mut fmt::Formatter<'_>, ctxt: &DisplayCtxt<'_>, data: DisplayData<'_>) -> fmt::Result {
        let need_brackets = data.bind_power > QUANT_BIND;
        // Within the body of the term of a quantified formula, we 
        // want to replace the quantified variables by their names
        // for this, we need to store the quantifier in the context
        let data = DisplayData {
            quant: Some(self),
            bind_power: QUANT_BIND,
            ..data
        };

        if need_brackets {
            write!(f, "(")?;
        }
        write!(f, "{}", self.kind.with_data(ctxt, data))?;
        for idx in 0..self.num_vars {
            let name = VarNames::get_name(&self.vars, idx);
            let ty = VarNames::get_type(&self.vars, idx);
            if idx != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{name}{ty}")?;
        }
        let sep = if ctxt.use_mathematical_symbols {
            "."
        } else {
            " ::"
        };
        write!(f, "{sep}")?;
        for child in data.children {
            write!(f, " {}", ctxt.parser[*child].with_data(ctxt, data))?;
        }
        if need_brackets {
            write!(f, ")")?;
        }
        Ok(())
    }
}

impl DisplayWithCtxt<DisplayCtxt<'_>, DisplayData<'_>> for QuantKind {
    fn fmt_with(&self, f: &mut fmt::Formatter<'_>, ctxt: &DisplayCtxt<'_>, _data: DisplayData<'_>) -> fmt::Result {
        if ctxt.use_mathematical_symbols {
            write!(f, "∀ ")?;
        } else {
            write!(f, "FORALL ")?;
        }
        if ctxt.display_quantifier_name {
            write!(f, "\"")?;
            match self {
                Self::Other(kind) => write!(f, "{kind}")?,
                Self::Lambda => write!(f, "<null>")?,
                Self::NamedQuant(name) => write!(f, "{name}")?,
                Self::UnnamedQuant { name, id } => write!(f, "{name}!{id}")?,
            };
            write!(f, "\" ")?;
        }
        Ok(())
    }
}

use decdnnf_rs::Literal;
use rug::Integer;
use std::io::{BufWriter, Stdout, StdoutLock, Write};

pub(crate) struct ModelWriter<W>
where
    W: Write,
{
    pattern: Vec<u8>,
    sign_location: Vec<usize>,
    buf: BufWriter<W>,
    n_enumerated: Integer,
    n_models: Integer,
    compact_display: bool,
    do_not_print: bool,
}

impl ModelWriter<StdoutLock<'static>> {
    pub fn new_locked(n_vars: usize, compact_display: bool, do_not_print: bool) -> Self {
        ModelWriter::new(
            n_vars,
            compact_display,
            do_not_print,
            BufWriter::with_capacity(128 * 1024, std::io::stdout().lock()),
        )
    }
}

impl ModelWriter<Stdout> {
    pub fn new_unlocked(n_vars: usize, compact_display: bool, do_not_print: bool) -> Self {
        ModelWriter::new(
            n_vars,
            compact_display,
            do_not_print,
            BufWriter::with_capacity(128 * 1024, std::io::stdout()),
        )
    }
}

impl<W> ModelWriter<W>
where
    W: Write,
{
    fn new(n_vars: usize, compact_display: bool, do_not_print: bool, buf: BufWriter<W>) -> Self {
        let mut sign_location = Vec::with_capacity(n_vars);
        let mut pattern = Vec::new();
        pattern.push(b'v');
        for i in 1..=n_vars {
            pattern.push(b' ');
            sign_location.push(pattern.len());
            pattern.push(b' ');
            pattern.extend_from_slice(format!("{i}").as_bytes());
        }
        pattern.extend_from_slice(" 0 \n".as_bytes());
        Self {
            pattern,
            sign_location,
            buf,
            n_enumerated: 0.into(),
            n_models: 0.into(),
            compact_display,
            do_not_print,
        }
    }

    pub fn write_model_ordered(&mut self, model: &[Option<Literal>]) {
        self.n_enumerated += 1;
        if self.do_not_print {
            self.n_models +=
                Integer::from(Integer::ONE << model.iter().filter(|opt| opt.is_none()).count());
            return;
        }
        let mut current_n_models = Integer::from(1);
        model
            .iter()
            .zip(self.sign_location.iter())
            .for_each(|(opt_l, o)| {
                if let Some(l) = opt_l {
                    if l.polarity() {
                        self.pattern[*o] = b' ';
                    } else {
                        self.pattern[*o] = b'-';
                    }
                } else {
                    self.pattern[*o] = b'*';
                    current_n_models <<= 1;
                }
            });
        let _ = self.buf.write_all(&self.pattern);
        self.n_models += current_n_models;
    }

    pub fn write_model_no_opt(&mut self, model: &[Literal]) {
        self.n_enumerated += 1;
        self.n_models += 1;
        if self.do_not_print {
            return;
        }
        for l in model {
            if l.polarity() {
                self.pattern[self.sign_location[l.var_index()]] = b' ';
            } else {
                self.pattern[self.sign_location[l.var_index()]] = b'-';
            }
        }
        let _ = self.buf.write_all(&self.pattern);
    }

    pub fn finalize(&mut self) {
        self.buf.flush().unwrap();
    }

    pub fn compact_display(&self) -> bool {
        self.compact_display
    }

    pub fn n_enumerated(&self) -> &Integer {
        &self.n_enumerated
    }

    pub fn n_models(&self) -> &Integer {
        &self.n_models
    }
}

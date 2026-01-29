// Loosely based on `../../../vendor/neovim/src/gen/cdoc_grammar.lua:65`

use std::fmt::{Display, Formatter};
use winnow::{
    ModalResult, Parser,
    ascii::{line_ending, space0, till_line_ending},
    combinator::{alt, fail, opt, repeat, seq, trace},
    dispatch,
    error::{ContextError, ErrMode, ParserError},
    stream::{AsChar, ContainsToken, Offset, Stream},
    token::{any, one_of, take_till, take_while},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CDocEvent<'i> {
    Description(&'i str),
    Attr(Attr<'i>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Attr<'i> {
    Param {
        dir: ParamDir,
        name: &'i str,
        desc: Option<&'i str>,
    },
    Return {
        desc: Option<&'i str>,
    },
    Deprecated,
    See {
        desc: &'i str,
    },
    Brief {
        desc: &'i str,
    },
    Note {
        desc: &'i str,
    },
    NoDoc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParamDir {
    None,
    In,
    Out,
    InOut,
}

pub fn cdoc<'i>(input: &mut &'i str) -> ModalResult<String> {
    let description = until(
        (till_line_ending, any).void(),
        (space0, peek_attr).void(),
        //
    )
    .parse_next(input)?;
    Ok(format!("{description}"))
}

pub fn cdoc_iterator<'a, 'i>(input: &'a mut &'i str) -> CDocIter<'i> {
    CDocIter {
        input,
        state: State::Init,
    }
}

#[allow(dead_code)]
pub struct CDocIter<'i> {
    input: &'i str,
    state: State<ErrMode<ContextError>>,
}

impl<'i> CDocIter<'i> {
    pub fn finish(self) -> Result<&'i str, ErrMode<ContextError>> {
        match self.state {
            State::Init | State::Attr | State::Done => Ok(self.input),
            State::Cut(err) => Err(err),
        }
    }
}

enum State<E> {
    Init,
    Attr,
    Done,
    Cut(E),
}

impl<'i> Iterator for CDocIter<'i> {
    type Item = CDocEvent<'i>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.state {
            State::Init => {
                let res = (|input| {
                    let description = until(
                        (till_line_ending, any).void(),
                        (space0, peek_attr).void(),
                        //
                    )
                    .parse_next(input)?;

                    Ok(CDocEvent::Description(description))
                })(&mut self.input);

                match res {
                    Ok(event) => {
                        self.state = State::Attr;
                        Some(event)
                    }
                    Err(ErrMode::Backtrack(_)) => {
                        self.state = State::Done;
                        None
                    }
                    Err(err) => {
                        self.state = State::Cut(err);
                        None
                    }
                }
            }
            State::Attr => {
                match (|i| { (space0, attr()).map(|(_, attr)| attr) }.parse_next(i))(
                    &mut self.input,
                ) {
                    Ok(event) => Some(CDocEvent::Attr(event)),
                    Err(ErrMode::Backtrack(_)) => {
                        self.state = State::Done;
                        None
                    }
                    Err(err) => {
                        self.state = State::Cut(err);
                        None
                    }
                }
            }
            State::Done | State::Cut(_) => None,
        }
    }
}

fn till_line_ending_incl<'i>(i: &mut &'i str) -> Result<(), ErrMode<ContextError>> {
    trace("till_line_ending_incl", |i: &mut &'i str| {
        let _ = till_line_ending.parse_next(i)?;
        match line_ending(i) {
            Ok(_) => Ok(()),
            Err(_) if i.is_empty() => Err(ErrMode::Backtrack(ContextError::new())),
            Err(err) => Err(err),
        }
    })
    .parse_next(i)
}

fn till_attr<'i>(input: &mut &'i str) -> Result<&'i str, ErrMode<ContextError>> {
    until(till_line_ending_incl, (space0, peek_attr).void()).parse_next(input)
}

fn until<'i, E>(
    mut chunk: impl Parser<&'i str, (), ErrMode<E>>,
    mut until: impl Parser<&'i str, (), ErrMode<E>>,
) -> impl Parser<&'i str, &'i str, ErrMode<E>>
where
    E: ParserError<&'i str>,
{
    // let attr_parser = peek_attr.parse_next(input);
    move |input: &mut &'i str| {
        let checkpoint = input.checkpoint();
        let mut next;

        loop {
            next = input.checkpoint();
            if let Ok(_) = until.parse_next(input) {
                input.reset(&checkpoint);
                break Ok(input.next_slice(next.offset_from(&checkpoint)));
            }
            match chunk.parse_next(input) {
                Ok(_) => {}
                Err(err) if err.is_backtrack() => {
                    input.reset(&checkpoint);
                    break Ok(input.next_slice(next.offset_from(&checkpoint)));
                }
                Err(err) => return Err(err),
            }
        }
    }
}

fn peek_attr<'i>(input: &mut &'i str) -> ModalResult<()> {
    trace(
        "peek_attr",
        (
            "@",
            dispatch!(take_till(1.., |c: char|c.is_whitespace());
                "param" => {},
                "return" | "returns" => (),
                "deprecated" => (),
                "see" => (),
                "brief" => (),
                "note" => (),
                "nodoc" => (),
            _ => fail,
            ),
        ),
    )
    .map(|(_, ())| ())
    .parse_next(input)
}

fn attr<'i>() -> impl Parser<&'i str, Attr<'i>, ErrMode<ContextError>> {
    use Attr::*;
    (
        "@",
        dispatch! { take_till(1.., |c: char| c.is_whitespace());
                "param" =>
                    seq!(Param {
                        dir: opt_or(param_dir, ParamDir::None),
                        _: ws1,
                        name: ident,
                        desc: opt((ws1, till_attr).map(|(_,desc)|desc)),
                    })
                ,
                "return" | "returns" =>
                    seq!(Return {
                        desc: opt((ws1, till_attr).map(|(_,desc)|desc)),
                    }),
                "deprecated" => ().value(Deprecated),
                "see" =>
                    seq!(See {
                        _: ws1,
                        _: (opt(ws1), "#", opt(ws1)),
                        desc: till_attr,
                    })
                ,
                "brief" =>
                    seq!(Brief {
                        _: ws1,
                        desc: till_attr,
                    })
                ,
                "note" =>
                    seq!(Note {
                        _: ws1,
                        desc: till_attr,
                    })
                ,
                "nodoc" => ().value(NoDoc),
                _ => fail,
        },
    )
        .map(|(_, attr)| attr)
}

fn param_dir<'i, E: ParserError<&'i str>>(input: &mut &'i str) -> Result<ParamDir, E> {
    use ParamDir::{In, InOut, Out};

    (
        "[",
        alt(("inout".value(InOut), "in".value(In), "out".value(Out))),
        "]",
    )
        .map(|(_, dir, _)| dir)
        .parse_next(input)
}

fn ws1<'i>(i: &mut &'i str) -> Result<&'i str, ErrMode<ContextError>> {
    take_while(1.., |c| AsChar::is_space(c) | AsChar::is_newline(c)).parse_next(i)
}

fn ident<'i, E: ParserError<&'i str>>(input: &mut &'i str) -> Result<&'i str, E> {
    fn is_letter<'i>() -> impl ContainsToken<<&'i str as Stream>::Token> {
        ('a'..='z', 'A'..'Z', '_', '$')
    }

    (
        one_of(is_letter()),
        repeat(0.., one_of((is_letter(), '0'..='9'))),
    )
        .map(|(_, ())| ())
        .take()
        .parse_next(input)
}

fn opt_or<F, I, O, E>(parser: F, default: O) -> impl Parser<I, O, E>
where
    F: Parser<I, O, E>,
    I: Stream,
    O: Clone,
    E: ParserError<I>,
{
    opt(parser).map(move |opt| opt.unwrap_or(default.clone()))
}

pub struct FormatComment<'a>(pub &'a str);

impl Display for FormatComment<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut input = self.0;
        let iter = cdoc_iterator(&mut input);
        let mut in_param = false;

        for ev in iter {
            match ev {
                CDocEvent::Description(desc) => f.write_str(desc)?,
                CDocEvent::Attr(Attr::Param { dir, name, desc }) => {
                    if !in_param {
                        in_param = true;
                        writeln!(f, "# Parameters")?;
                    }

                    write!(
                        f,
                        "- `{name}{dir}`",
                        dir = match dir {
                            ParamDir::None => "",
                            ParamDir::In => "[in]",
                            ParamDir::Out => "[out]",
                            ParamDir::InOut => "[inout]",
                        },
                    )?;
                    match desc {
                        Some(desc) => write!(f, " {desc}")?,
                        None => writeln!(f)?,
                    }
                }
                CDocEvent::Attr(Attr::Return { desc: None }) => {}
                CDocEvent::Attr(Attr::Return { desc: Some(desc) }) => {
                    writeln!(f, "# Return value\n\n{desc}")?;
                }
                CDocEvent::Attr(Attr::Deprecated) => {}
                CDocEvent::Attr(Attr::See { desc }) => {
                    writeln!(f, "See: {desc}")?;
                }
                CDocEvent::Attr(Attr::Brief { desc }) => {
                    writeln!(f, "Brief: {desc}")?;
                }
                CDocEvent::Attr(Attr::Note { desc }) => {
                    writeln!(f, "Note: {desc}")?;
                }
                CDocEvent::Attr(Attr::NoDoc) => {}
            }
        }
        Ok(())
    }
}

#[test]
fn test_parse_comment() {
    let mut input = include_str!("../test-comment.txt");
    let mut iter = cdoc_iterator(&mut input);
    assert_eq!(
        Some(CDocEvent::Description(
            " Get all autocommands that match the corresponding {opts}.\n\n These examples will get autocommands matching ALL the given criteria:\n\n ```lua\n -- Matches all criteria\n autocommands = vim.api.nvim_get_autocmds({\n   group = 'MyGroup',\n   event = {'BufEnter', 'BufWinEnter'},\n   pattern = {'*.c', '*.h'}\n })\n\n -- All commands from one group\n autocommands = vim.api.nvim_get_autocmds({\n   group = 'MyGroup',\n })\n ```\n\n NOTE: When multiple patterns or events are provided, it will find all the autocommands that\n match any combination of them.\n\n"
        )),
        iter.next()
    );
    assert_eq!(
        Some(CDocEvent::Attr(Attr::Param {
            dir: ParamDir::None,
            name: "opts",
            desc: Some(
                "Dict with at least one of the following:\n             - buffer: (integer) Buffer number or list of buffer numbers for buffer local autocommands\n             |autocmd-buflocal|. Cannot be used with {pattern}\n             - event: (vim.api.keyset.events|vim.api.keyset.events[])\n               event or events to match against |autocmd-events|.\n             - id: (integer) Autocommand ID to match.\n             - group: (string|table) the autocommand group name or id to match against.\n             - pattern: (string|table) pattern or patterns to match against |autocmd-pattern|.\n             Cannot be used with {buffer}\n"
            )
        })),
        iter.next()
    );
    assert_eq!(
        Some(CDocEvent::Attr(Attr::Return {
            desc: Some(
                "Array of autocommands matching the criteria, with each item\n             containing the following fields:\n             - buffer: (integer) the buffer number.\n             - buflocal: (boolean) true if the autocommand is buffer local.\n             - command: (string) the autocommand command. Note: this will be empty if a callback is set.\n             - callback: (function|string|nil): Lua function or name of a Vim script function\n               which is executed when this autocommand is triggered.\n             - desc: (string) the autocommand description.\n             - event: (vim.api.keyset.events) the autocommand event.\n             - id: (integer) the autocommand id (only when defined with the API).\n             - group: (integer) the autocommand group id.\n             - group_name: (string) the autocommand group name.\n             - once: (boolean) whether the autocommand is only run once.\n             - pattern: (string) the autocommand pattern.\n               If the autocommand is buffer local |autocmd-buffer-local|:\n"
            )
        })),
        iter.next()
    );
    assert_eq!(None, iter.next());
    assert_eq!(Ok(""), iter.finish());
}

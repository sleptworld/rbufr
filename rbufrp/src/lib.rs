use pyo3::prelude::*;

#[pymodule]
mod _core {
    use librbufr::{
        Decoder,
        block::{BUFRFile as IB, MessageBlock as IM},
        decoder::BUFRParsed as _BUFRParsed,
        errors::Error,
        get_tables_base_path, parse, set_tables_base_path,
    };
    use pyo3::prelude::*;

    #[pyfunction]
    fn set_tables_path(path: &str) -> PyResult<()> {
        set_tables_base_path(path);
        Ok(())
    }

    #[pyfunction]
    fn get_tables_path() -> PyResult<String> {
        let path = get_tables_base_path();
        Ok(path.to_string_lossy().to_string())
    }

    #[pyclass]
    struct BUFRDecoder {}

    #[pymethods]
    impl BUFRDecoder {
        #[new]
        fn new() -> Self {
            BUFRDecoder {}
        }

        fn decode(&self, file_path: &str) -> PyResult<BUFRFile> {
            let parsed = parse(file_path).map_err(|e| match e {
                Error::Io(io_err) => {
                    PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("IO Error: {}", io_err))
                }

                Error::ParseError(parse_err) => PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    format!("Parse Error: {}", parse_err),
                ),

                Error::Nom(nom_err) => PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Parse Error: {}",
                    nom_err
                )),

                _ => PyErr::new::<pyo3::exceptions::PyException, _>(
                    "An unknown error occurred during BUFR decoding.",
                ),
            })?;

            Ok(BUFRFile(parsed))
        }

        fn parse_message(&self, message: &BUFRMessage) -> PyResult<BUFRParsed> {
            self._parse_message(message).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyException, _>(format!(
                    "Error parsing BUFR message: {}",
                    e
                ))
            })
        }
    }

    impl BUFRDecoder {
        fn _parse_message(&self, message: &BUFRMessage) -> librbufr::errors::Result<BUFRParsed> {
            let _message = &message.message;
            let mut decoder = Decoder::from_message(_message)?;
            let record = decoder.decode(_message)?.into_owned();
            Ok(BUFRParsed(record))
        }
    }

    #[pyclass]
    struct BUFRFile(IB);

    #[pymethods]
    impl BUFRFile {
        fn __repr__(&self) -> String {
            format!("BUFRFile with {} messages", self.0.message_count())
        }

        fn message_count(&self) -> usize {
            self.0.message_count()
        }

        fn get_message(&self, index: usize) -> PyResult<BUFRMessage> {
            let message = self.0.message_at(index).ok_or_else(|| {
                PyErr::new::<pyo3::exceptions::PyIndexError, _>("Message index out of range")
            })?;

            Ok(BUFRMessage {
                message: message.clone(),
            })
        }
    }

    #[pyclass]
    struct BUFRMessage {
        message: IM,
    }

    #[pymethods]
    impl BUFRMessage {
        fn __repr__(&self) -> String {
            format!("{}", self.message)
        }

        fn version(&self) -> u8 {
            self.message.version()
        }
    }

    #[pyclass]
    struct BUFRParsed(_BUFRParsed<'static>);

    #[pymethods]
    impl BUFRParsed {
        fn __repr__(&self) -> String {
            format!("{}", &self.0)
        }
    }
}

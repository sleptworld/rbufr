use pyo3::prelude::*;
#[pymodule]
mod _core {
    use librbufr::{
        Decoder,
        block::{BUFRFile as IB, MessageBlock as IM},
        decoder::{BUFRParsed as _BUFRParsed, BUFRRecord as _BUFRRecord},
        errors::Error,
        get_tables_base_path, parse, set_tables_base_path,
    };
    use pyo3::{IntoPyObjectExt, prelude::*, types::PyList};

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

        fn decode(&self, bytes: &[u8]) -> PyResult<BUFRFile> {
            let parsed = parse(bytes).map_err(|e| match e {
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

            Ok(BUFRFile(parsed, 0))
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
            Ok(BUFRParsed {
                inner: record,
                iter_index: 0,
            })
        }
    }

    #[pyclass]
    struct BUFRFile(IB, usize);

    #[pymethods]
    impl BUFRFile {
        fn __repr__(&self) -> String {
            format!("BUFRFile with {} messages", self.0.message_count())
        }

        fn __len__(&self) -> usize {
            self.0.message_count()
        }

        fn __iter__(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
            slf.1 = 0;
            slf
        }

        fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<BUFRMessage> {
            let current_index = slf.1;
            let message_count = slf.0.message_count();

            if current_index < message_count {
                slf.1 += 1;
                let message = slf.0.message_at(current_index).unwrap().clone();
                Some(BUFRMessage { message })
            } else {
                None
            }
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

        fn section2(&self) -> Option<Section2> {
            self.message
                .section2()
                .map(|s| Section2 { inner: s.clone() })
        }
    }

    #[pyclass]
    struct Section2 {
        inner: librbufr::structs::versions::Section2,
    }

    #[pymethods]
    impl Section2 {
        fn len(&self) -> usize {
            self.inner.length
        }

        fn is_empty(&self) -> bool {
            self.inner.length == 0
        }

        fn get_raw_bytes(&self) -> Vec<u8> {
            self.inner.data.clone()
        }
    }

    #[pyclass]
    struct BUFRParsed {
        inner: _BUFRParsed<'static>,
        #[pyo3(get)]
        iter_index: usize,
    }

    #[pymethods]
    impl BUFRParsed {
        fn __repr__(&self) -> String {
            format!("{}", &self.inner)
        }

        fn __iter__(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
            slf.iter_index = 0;
            slf
        }

        fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<BUFRRecord> {
            let current_index = slf.iter_index;
            let record_count = slf.inner.record_count();

            if current_index < record_count {
                slf.iter_index += 1;
                let record = slf.inner.records()[current_index].into_owned();
                Some(BUFRRecord(record))
            } else {
                None
            }
        }

        fn __len__(&self) -> usize {
            self.inner.record_count()
        }

        fn __getitem__(&self, index: isize) -> PyResult<BUFRRecord> {
            let len = self.inner.record_count() as isize;

            let idx = if index < 0 {
                (len + index) as usize
            } else {
                index as usize
            };

            if idx < self.inner.record_count() {
                let record = self.inner.records()[idx].into_owned();
                Ok(BUFRRecord(record))
            } else {
                Err(PyErr::new::<pyo3::exceptions::PyIndexError, _>(
                    "Index out of range",
                ))
            }
        }

        fn record_count(&self) -> usize {
            self.inner.record_count()
        }

        fn get_record(&self, key: &str) -> Vec<BUFRRecord> {
            let mut records = Vec::new();
            for record in self.inner.records() {
                if let Some(name) = &record.name {
                    if name == key {
                        records.push(BUFRRecord(record.into_owned()));
                    }
                }
            }
            records
        }
    }

    #[pyclass]
    struct BUFRRecord(_BUFRRecord<'static>);

    #[pymethods]
    impl BUFRRecord {
        fn __repr__(&self) -> String {
            format!("{}", &self.0)
        }

        fn key(&self) -> Option<String> {
            self.0.name.as_ref().map(|s| s.to_string())
        }

        fn value<'py>(&self, py: Python<'py>) -> Py<PyAny> {
            use librbufr::BUFRData::*;
            use librbufr::Value::*;
            use numpy::PyArray1;
            match &self.0.values {
                Repeat(vs) => {
                    let list = PyList::empty(py);

                    for v in vs {
                        match v {
                            Number(n) => {
                                list.append(n).unwrap();
                            }
                            Missing => {
                                list.append(py.None()).unwrap();
                            }
                            String(s) => {
                                list.append(s).unwrap();
                            }
                        }
                    }
                    list.into_py_any(py).unwrap()
                }
                Single(v) => match v {
                    Number(n) => n.into_py_any(py).unwrap(),
                    Missing => py.None().into_py_any(py).unwrap(),
                    String(s) => s.into_py_any(py).unwrap(),
                },
                Array(a) => {
                    let array = PyArray1::from_vec(py, a.clone());
                    array.into_py_any(py).unwrap()
                }
            }
        }
    }
}

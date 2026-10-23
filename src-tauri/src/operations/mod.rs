//! The contract of operations (D-498, `docs/design/contrato-de-operaciones.md`):
//! the one list of what can be asked of the base, by the chat, by the
//! buttons of the screens and, in the next step, by the local API and the
//! MCP. Each operation says what it asks for, what it changes, whether a
//! person confirms it and how it is undone; the chat's instructions and
//! its tools are written from this list, and so will the MCP's tools be.
//!
//! Nothing here depends on Tauri but `commands.rs`: the rest takes a
//! `DocBase` and is tested with `DocBase::in_memory()`.
//!
//! - `read.rs`: the operations that only read.
//! - `resolve.rs`: from what the model named to what the base has.
//! - `card.rs`: the card a person confirms, and its fingerprint.
//! - `apply.rs`: applying and undoing what a person confirmed.
//! - `flow.rs`: propose, choose, confirm, cancel and undo.
//! - `log.rs`: the record of operations (`operation_log`).
//! - `prompt.rs`: the chat's instructions, in Spanish, from the list.
//! - `words.rs`: the words of the profession the list is written with.

pub mod apply;
pub mod card;
pub mod commands;
pub mod flow;
pub mod log;
pub mod prompt;
pub mod read;
pub mod resolve;
pub mod words;

#[cfg(test)]
mod tests;

use serde::Serialize;
use serde_json::{json, Map, Value};

pub use card::Card;
pub use flow::Proposal;
pub use words::Words;

/// Whether an operation only reads or changes something.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Read,
    Change,
}

/// How a person confirms an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Confirmation {
    /// A reading, or the clip of a page (the click in the extension is the
    /// person's decision).
    None,
    /// A card with what it touches.
    Card,
    /// A card that says what it costs before (questions to the model).
    CardWithCost,
}

/// How what an operation changed is undone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "why", rename_all = "snake_case")]
pub enum Undo {
    /// Nothing to undo: it only reads.
    Nothing,
    /// What it touched goes back to how it was.
    Restore,
    /// The decision of a person it made is undone.
    DecisionUndo,
    /// What it created is discarded (it stays in the record).
    Discard,
    /// It cannot be undone, and the card says it before.
    Not(&'static str),
}

/// Who can ask for an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Caller {
    /// The chat of the app: changes wait for a person on a card.
    Chat,
    /// A button of a screen: it is the person.
    Screen,
    /// The web clipper.
    Clipper,
    /// A program with the permission to read.
    ProgramRead,
    /// A program with the permission to ask for changes (they wait in
    /// Atención).
    ProgramChange,
}

/// The shape of one piece of data an operation asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    Text,
    Number,
    Bool,
    /// A list of names or identifiers.
    TextList,
    /// One of these words.
    OneOf(&'static [&'static str]),
    /// Names and values («campo: valor»).
    Values,
}

/// One piece of data an operation asks for, with its name in Spanish.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Field {
    pub name: &'static str,
    pub kind: FieldKind,
    pub required: bool,
    /// What it is, with the words of the profession (`{caso}`, `{ficha}`).
    pub about: &'static str,
}

/// One operation of the contract.
#[derive(Debug, Clone, Copy)]
pub struct Operation {
    /// The same for the chat, the API and the MCP.
    pub name: &'static str,
    /// What it does, with the words of the profession.
    pub summary: &'static str,
    pub kind: Kind,
    pub fields: &'static [Field],
    /// An example of its data, as JSON (checked against its fields).
    pub example: &'static str,
    /// What it changes, in words; empty for a reading.
    pub changes: &'static str,
    pub confirmation: Confirmation,
    pub undo: Undo,
    pub callers: &'static [Caller],
    /// Built and offered; one that is not yet is kept in the list so the
    /// contract is whole, and nobody can ask for it.
    pub ready: bool,
}

const fn field(name: &'static str, kind: FieldKind, required: bool, about: &'static str) -> Field {
    Field { name, kind, required, about }
}

const READERS: &[Caller] = &[Caller::Chat, Caller::ProgramRead, Caller::ProgramChange];
const CHANGERS: &[Caller] = &[Caller::Chat, Caller::Screen, Caller::ProgramChange];

const DOCUMENTS: &str = "Los documentos: cada uno por su identificador (doc-…) o por su nombre; «este documento» o «estos» son los adjuntos del mensaje.";
const MATTER: &str = "El {caso}: su identificador (mat-…) o su nombre.";
const PART: &str = "Qué parte leer de un texto largo (1, 2…); sin ella, la primera.";

/// The contract: every operation, readings first.
pub static OPERATIONS: &[Operation] = &[
    Operation {
        name: "buscar",
        summary: "Busca documentos, personas, organizaciones y {casos}; devuelve cada uno con su identificador, su título, su lugar y el pasaje encontrado.",
        kind: Kind::Read,
        fields: &[
            field("consulta", FieldKind::Text, true, "Qué buscar, en pocas palabras."),
            field("cantidad", FieldKind::Number, false, "Cuántos resultados, hasta 10."),
            field("tipo", FieldKind::OneOf(&["documentos", "fichas", "casos"]), false, "Solo documentos, solo {fichas} o solo {casos}."),
        ],
        example: r#"{"consulta":"cartas documento marzo","cantidad":5}"#,
        changes: "",
        confirmation: Confirmation::None,
        undo: Undo::Nothing,
        callers: READERS,
        ready: true,
    },
    Operation {
        name: "buscar_pasajes",
        summary: "Busca pasajes literales en el texto de los documentos; se puede limitar a ciertos documentos o a los de un {caso}.",
        kind: Kind::Read,
        fields: &[
            field("consulta", FieldKind::Text, true, "Las palabras del pasaje."),
            field("documentos", FieldKind::TextList, false, "Solo en estos documentos (doc-…)."),
            field("caso", FieldKind::Text, false, "Solo en los documentos de este {caso}."),
            field("cantidad", FieldKind::Number, false, "Cuántos pasajes, hasta 10."),
        ],
        example: r#"{"consulta":"plazo de preaviso","caso":"mat-p-1a2b"}"#,
        changes: "",
        confirmation: Confirmation::None,
        undo: Undo::Nothing,
        callers: READERS,
        ready: true,
    },
    Operation {
        name: "leer",
        summary: "Lee el artículo de un documento, una {ficha} o un {caso}, por partes si es largo («parte 2 de 5»).",
        kind: Kind::Read,
        fields: &[
            field("id", FieldKind::Text, true, "El identificador (doc-…, ent-…, mat-…) o el nombre."),
            field("parte", FieldKind::Number, false, PART),
        ],
        example: r#"{"id":"doc-41"}"#,
        changes: "",
        confirmation: Confirmation::None,
        undo: Undo::Nothing,
        callers: READERS,
        ready: true,
    },
    Operation {
        name: "leer_texto",
        summary: "Lee el texto original de un documento (el que se cita), por partes.",
        kind: Kind::Read,
        fields: &[
            field("documento", FieldKind::Text, true, "El documento: doc-… o su nombre."),
            field("parte", FieldKind::Number, false, PART),
        ],
        example: r#"{"documento":"doc-41","parte":2}"#,
        changes: "",
        confirmation: Confirmation::None,
        undo: Undo::Nothing,
        callers: READERS,
        ready: true,
    },
    Operation {
        name: "relaciones",
        summary: "Las relaciones que la base conoce de una persona, organización, {caso} o documento.",
        kind: Kind::Read,
        fields: &[field("nombre", FieldKind::Text, true, "El nombre de quien o de qué.")],
        example: r#"{"nombre":"Ejemplo S.A."}"#,
        changes: "",
        confirmation: Confirmation::None,
        undo: Undo::Nothing,
        callers: READERS,
        ready: true,
    },
    Operation {
        name: "ver_documento",
        summary: "Los datos de un documento: tipo, lugar, {casos} en que está, personas y organizaciones con su papel.",
        kind: Kind::Read,
        fields: &[field("documento", FieldKind::Text, true, "El documento: doc-… o su nombre.")],
        example: r#"{"documento":"doc-41"}"#,
        changes: "",
        confirmation: Confirmation::None,
        undo: Undo::Nothing,
        callers: READERS,
        ready: true,
    },
    Operation {
        name: "ver_ficha",
        summary: "Los datos de una {ficha}: cómo se escribe, sus identificadores, sus documentos, sus {casos} y sus decisiones.",
        kind: Kind::Read,
        fields: &[field("ficha", FieldKind::Text, true, "La {ficha}: ent-… o su nombre.")],
        example: r#"{"ficha":"ent-a1b2"}"#,
        changes: "",
        confirmation: Confirmation::None,
        undo: Undo::Nothing,
        callers: READERS,
        ready: true,
    },
    Operation {
        name: "ver_caso",
        summary: "Un {caso} con su estado, sus documentos (firmes y propuestos), sus partes y sus decisiones.",
        kind: Kind::Read,
        fields: &[field("caso", FieldKind::Text, true, MATTER)],
        example: r#"{"caso":"mat-p-1a2b"}"#,
        changes: "",
        confirmation: Confirmation::None,
        undo: Undo::Nothing,
        callers: READERS,
        ready: true,
    },
    Operation {
        name: "listar_casos",
        summary: "Los {casos}, con su estado y cuántos documentos tienen.",
        kind: Kind::Read,
        fields: &[field("incluir_cerrados", FieldKind::Bool, false, "También los {casos} cerrados y archivados.")],
        example: r#"{}"#,
        changes: "",
        confirmation: Confirmation::None,
        undo: Undo::Nothing,
        callers: READERS,
        ready: true,
    },
    Operation {
        name: "listar_plantillas",
        summary: "Las plantillas, con su tipo, sus documentos y sus campos.",
        kind: Kind::Read,
        fields: &[],
        example: r#"{}"#,
        changes: "",
        confirmation: Confirmation::None,
        undo: Undo::Nothing,
        callers: READERS,
        ready: true,
    },
    Operation {
        name: "estado_pedido",
        summary: "En qué quedó un cambio que pidió un programa: pendiente, hecho, cancelado o vencido.",
        kind: Kind::Read,
        fields: &[field("pedido", FieldKind::Number, true, "El número de pedido.")],
        example: r#"{"pedido":12}"#,
        changes: "",
        confirmation: Confirmation::None,
        undo: Undo::Nothing,
        callers: &[Caller::ProgramRead, Caller::ProgramChange],
        ready: true,
    },
    Operation {
        name: "caso_agregar_documentos",
        summary: "Pone documentos en un {caso}.",
        kind: Kind::Change,
        fields: &[field("caso", FieldKind::Text, true, MATTER), field("documentos", FieldKind::TextList, true, DOCUMENTS)],
        example: r#"{"caso":"mat-p-1a2b","documentos":["doc-41","doc-57"]}"#,
        changes: "Los documentos pasan a ser del {caso}; un {caso} propuesto queda confirmado.",
        confirmation: Confirmation::Card,
        undo: Undo::Restore,
        callers: CHANGERS,
        ready: true,
    },
    Operation {
        name: "caso_sacar_documentos",
        summary: "Saca documentos de un {caso}.",
        kind: Kind::Change,
        fields: &[field("caso", FieldKind::Text, true, MATTER), field("documentos", FieldKind::TextList, true, DOCUMENTS)],
        example: r#"{"caso":"mat-p-1a2b","documentos":["doc-57"]}"#,
        changes: "Dejan de ser del {caso} y ninguna pasada los vuelve a poner; a un {caso} propuesto no se le puede sacar el último.",
        confirmation: Confirmation::Card,
        undo: Undo::Restore,
        callers: CHANGERS,
        ready: true,
    },
    Operation {
        name: "caso_crear",
        summary: "Crea un {caso} con un nombre y, si se quiere, documentos.",
        kind: Kind::Change,
        fields: &[
            field("nombre", FieldKind::Text, true, "El nombre del {caso} nuevo."),
            field("documentos", FieldKind::TextList, false, DOCUMENTS),
        ],
        example: r#"{"nombre":"Prueba c/ Ejemplo S.A.","documentos":["doc-41"]}"#,
        changes: "Un {caso} nuevo, confirmado, creado a mano.",
        confirmation: Confirmation::Card,
        undo: Undo::Discard,
        callers: CHANGERS,
        ready: true,
    },
    Operation {
        name: "caso_renombrar",
        summary: "Le cambia el nombre a un {caso}.",
        kind: Kind::Change,
        fields: &[field("caso", FieldKind::Text, true, MATTER), field("nombre", FieldKind::Text, true, "El nombre nuevo.")],
        example: r#"{"caso":"mat-p-1a2b","nombre":"Prueba c/ Ejemplo S.A. (despido)"}"#,
        changes: "El nombre queda fijo aunque la app reagrupe.",
        confirmation: Confirmation::Card,
        undo: Undo::Restore,
        callers: CHANGERS,
        ready: true,
    },
    Operation {
        name: "caso_decidir",
        summary: "Confirma, descarta o vuelve a propuesto un {caso} que propuso la app.",
        kind: Kind::Change,
        fields: &[
            field("caso", FieldKind::Text, true, MATTER),
            field("decision", FieldKind::OneOf(&["confirmar", "descartar", "volver_a_propuesto"]), true, "Qué decidir."),
        ],
        example: r#"{"caso":"mat-1a2b3c","decision":"confirmar"}"#,
        changes: "El {caso} queda confirmado, descartado o propuesto.",
        confirmation: Confirmation::Card,
        undo: Undo::Restore,
        callers: CHANGERS,
        ready: true,
    },
    Operation {
        name: "caso_cambiar_estado",
        summary: "Cierra, archiva o reabre un {caso}.",
        kind: Kind::Change,
        fields: &[
            field("caso", FieldKind::Text, true, MATTER),
            field("estado", FieldKind::OneOf(&["abierto", "cerrado", "archivado"]), true, "El estado nuevo."),
        ],
        example: r#"{"caso":"mat-p-1a2b","estado":"cerrado"}"#,
        changes: "Cerrado o archivado: sale de las listas y de Atención, sigue en la búsqueda con la marca y no recibe propuestas nuevas.",
        confirmation: Confirmation::Card,
        undo: Undo::Restore,
        callers: CHANGERS,
        ready: true,
    },
    Operation {
        name: "fichas_unir",
        summary: "Dice que dos {fichas} son la misma persona u organización.",
        kind: Kind::Change,
        fields: &[field("fichas", FieldKind::TextList, true, "Las dos {fichas}: ent-… o sus nombres.")],
        example: r#"{"fichas":["ent-a1b2","ent-c3d4"]}"#,
        changes: "Pasan a ser una, con todos sus documentos; ninguna pasada las separa.",
        confirmation: Confirmation::Card,
        undo: Undo::DecisionUndo,
        callers: CHANGERS,
        ready: true,
    },
    Operation {
        name: "fichas_separar",
        summary: "Dice que dos {fichas} no son la misma, o que una {ficha} en un documento es otra.",
        kind: Kind::Change,
        fields: &[
            field("fichas", FieldKind::TextList, false, "Las dos {fichas} que no son la misma."),
            field("ficha", FieldKind::Text, false, "O una {ficha}…"),
            field("documento", FieldKind::Text, false, "…y el documento donde es otra."),
        ],
        example: r#"{"fichas":["ent-a1b2","ent-c3d4"]}"#,
        changes: "Quedan separadas para siempre (salvo evidencia fuerte y nueva).",
        confirmation: Confirmation::Card,
        undo: Undo::DecisionUndo,
        callers: CHANGERS,
        ready: true,
    },
    Operation {
        name: "grupo_crear",
        summary: "Dice que dos o más sociedades son del mismo grupo.",
        kind: Kind::Change,
        fields: &[field("sociedades", FieldKind::TextList, true, "Las sociedades: ent-… o sus nombres.")],
        example: r#"{"sociedades":["Ejemplo S.A.","Ejemplo Logística S.R.L."]}"#,
        changes: "Se crea la {ficha} del grupo, que se ve en cada sociedad.",
        confirmation: Confirmation::Card,
        undo: Undo::DecisionUndo,
        callers: CHANGERS,
        ready: false,
    },
    Operation {
        name: "documento_releer",
        summary: "Vuelve a leer documentos con el modelo.",
        kind: Kind::Change,
        fields: &[field("documentos", FieldKind::TextList, true, DOCUMENTS)],
        example: r#"{"documentos":["doc-41"]}"#,
        changes: "Se rehacen sus lecturas; cuesta preguntas al modelo.",
        confirmation: Confirmation::CardWithCost,
        undo: Undo::Not("Volver a leer no se deshace: las preguntas al modelo ya se hicieron."),
        callers: CHANGERS,
        ready: false,
    },
    Operation {
        name: "plantilla_generar",
        summary: "Prepara un documento desde una plantilla, para un {caso}: abre la pantalla de generar completada.",
        kind: Kind::Change,
        fields: &[
            field("plantilla", FieldKind::Text, true, "La plantilla: su nombre o el de su documento principal."),
            field("caso", FieldKind::Text, false, MATTER),
            field("valores", FieldKind::Values, false, "Los valores de los campos, por su nombre."),
        ],
        example: r#"{"plantilla":"Carta documento de intimación","caso":"mat-p-1a2b","valores":{"Fecha":"12 de marzo"}}"#,
        changes: "Nada hasta que la persona toca «Generar» en la pantalla de generar.",
        confirmation: Confirmation::Card,
        undo: Undo::Not("El Word generado queda en «Generados»: nada borra archivos."),
        callers: CHANGERS,
        ready: false,
    },
    Operation {
        name: "consulta_guardar",
        summary: "Guarda la respuesta del chat como consulta.",
        kind: Kind::Change,
        fields: &[
            field("titulo", FieldKind::Text, true, "Un título corto."),
            field("respuesta", FieldKind::Text, true, "La respuesta, entera."),
        ],
        example: r#"{"titulo":"Quién alquila el depósito","respuesta":"La locataria es Ana Prueba (doc-41)."}"#,
        changes: "Una consulta guardada más.",
        confirmation: Confirmation::Card,
        undo: Undo::Discard,
        callers: CHANGERS,
        ready: true,
    },
    Operation {
        name: "recorte_guardar",
        summary: "Guarda una página web en «Recortes web».",
        kind: Kind::Change,
        fields: &[
            field("titulo", FieldKind::Text, true, "El título de la página."),
            field("direccion", FieldKind::Text, true, "Su dirección."),
            field("texto", FieldKind::Text, true, "Su texto."),
        ],
        example: r#"{"titulo":"Fallo de prueba","direccion":"https://example.org/fallo","texto":"…"}"#,
        changes: "Un archivo nuevo en la carpeta de trabajo, que la base toma.",
        confirmation: Confirmation::None,
        undo: Undo::Not("El archivo queda en su carpeta: no se deshace desde la app."),
        callers: &[Caller::Clipper],
        ready: false,
    },
];

/// Every operation of the contract (what the local API and the MCP of the
/// next step list).
#[cfg_attr(not(test), allow(dead_code))]
pub fn all() -> &'static [Operation] {
    OPERATIONS
}

/// One operation by its name.
pub fn find(name: &str) -> Option<&'static Operation> {
    let name = name.trim();
    OPERATIONS.iter().find(|op| op.name == name)
}

/// The operations a caller can ask for now.
pub fn offered_to(caller: Caller) -> impl Iterator<Item = &'static Operation> {
    OPERATIONS.iter().filter(move |op| op.ready && op.callers.contains(&caller))
}

/// The most elements a card holds, and the most before a second
/// confirmation (D-504, 4.3).
pub const MAX_CARD_ITEMS: usize = 200;
pub const SECOND_CONFIRMATION_OVER: usize = 10;
/// The most candidates «¿Cuál?» shows.
pub const MAX_CANDIDATES: usize = 5;

impl Operation {
    /// The JSON Schema of its data, with the names in Spanish and the
    /// words of the profession: the same for the chat, the API and the MCP.
    pub fn input_schema(&self, words: &Words) -> Value {
        let mut properties = Map::new();
        let mut required = Vec::new();
        for f in self.fields {
            let about = words.fill(f.about);
            let schema = match f.kind {
                FieldKind::Text => json!({ "type": "string", "description": about }),
                FieldKind::Number => json!({ "type": "integer", "description": about }),
                FieldKind::Bool => json!({ "type": "boolean", "description": about }),
                FieldKind::TextList => json!({ "type": "array", "items": { "type": "string" }, "description": about }),
                FieldKind::OneOf(values) => json!({ "type": "string", "enum": values, "description": about }),
                FieldKind::Values => json!({ "type": "object", "additionalProperties": { "type": "string" }, "description": about }),
            };
            properties.insert(f.name.to_string(), schema);
            if f.required {
                required.push(f.name);
            }
        }
        json!({ "type": "object", "properties": properties, "required": required, "additionalProperties": false })
    }

    /// Its data, checked against its fields and put in shape: a single
    /// name where a list goes becomes a list of one, a number written as
    /// text becomes a number. Fields it does not have are dropped. Says in
    /// Spanish what is missing or wrong.
    pub fn check(&self, data: &Value) -> Result<Value, String> {
        let empty = Map::new();
        let given = match data {
            Value::Object(map) => map,
            Value::Null => &empty,
            _ => return Err(format!("Los datos de {} van como un objeto JSON", self.name)),
        };
        let mut out = Map::new();
        for f in self.fields {
            let value = given.get(f.name).filter(|v| !v.is_null());
            let Some(value) = value else {
                if f.required {
                    return Err(format!("A {} le falta «{}»", self.name, f.name));
                }
                continue;
            };
            let shaped = match f.kind {
                FieldKind::Text => match value {
                    Value::String(s) if !s.trim().is_empty() => Value::String(s.trim().to_string()),
                    Value::Number(n) => Value::String(n.to_string()),
                    Value::String(_) if !f.required => continue,
                    _ => return Err(format!("«{}» de {} tiene que ser un texto", f.name, self.name)),
                },
                FieldKind::Number => match value {
                    Value::Number(n) if n.as_i64().is_some() => value.clone(),
                    Value::String(s) if s.trim().parse::<i64>().is_ok() => json!(s.trim().parse::<i64>().unwrap_or_default()),
                    _ => return Err(format!("«{}» de {} tiene que ser un número", f.name, self.name)),
                },
                FieldKind::Bool => match value {
                    Value::Bool(_) => value.clone(),
                    Value::String(s) if matches!(s.trim(), "true" | "si" | "sí") => Value::Bool(true),
                    Value::String(s) if matches!(s.trim(), "false" | "no") => Value::Bool(false),
                    _ => return Err(format!("«{}» de {} tiene que ser verdadero o falso", f.name, self.name)),
                },
                FieldKind::TextList => {
                    let items: Vec<String> = match value {
                        Value::String(s) => vec![s.trim().to_string()],
                        Value::Array(items) => items
                            .iter()
                            .map(|item| match item {
                                Value::String(s) => Ok(s.trim().to_string()),
                                Value::Number(n) => Ok(n.to_string()),
                                _ => Err(format!("«{}» de {} es una lista de nombres o identificadores", f.name, self.name)),
                            })
                            .collect::<Result<_, _>>()?,
                        _ => return Err(format!("«{}» de {} es una lista de nombres o identificadores", f.name, self.name)),
                    };
                    let mut seen = Vec::new();
                    for item in items.into_iter().filter(|i| !i.is_empty()) {
                        if !seen.contains(&item) {
                            seen.push(item);
                        }
                    }
                    if seen.is_empty() {
                        if f.required {
                            return Err(format!("A {} le falta «{}»", self.name, f.name));
                        }
                        continue;
                    }
                    json!(seen)
                }
                FieldKind::OneOf(values) => match value {
                    Value::String(s) if values.contains(&s.trim()) => Value::String(s.trim().to_string()),
                    _ => return Err(format!("«{}» de {} es una de estas palabras: {}", f.name, self.name, values.join(", "))),
                },
                FieldKind::Values => match value {
                    Value::Object(map) => {
                        let mut values = Map::new();
                        for (k, v) in map {
                            let text = match v {
                                Value::String(s) => s.clone(),
                                Value::Null => continue,
                                other => other.to_string(),
                            };
                            values.insert(k.trim().to_string(), Value::String(text));
                        }
                        Value::Object(values)
                    }
                    _ => return Err(format!("«{}» de {} va como un objeto de nombres y valores", f.name, self.name)),
                },
            };
            out.insert(f.name.to_string(), shaped);
        }
        Ok(Value::Object(out))
    }
}

/// The contract as a program reads it (Ajustes, the tests, the MCP of the
/// next step): each operation with its words already filled.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationView {
    pub name: String,
    pub summary: String,
    pub kind: Kind,
    pub changes: String,
    pub confirmation: Confirmation,
    pub undo: Undo,
    pub callers: Vec<Caller>,
    pub ready: bool,
    pub input_schema: Value,
    pub example: Value,
}

pub fn contract(words: &Words) -> Vec<OperationView> {
    OPERATIONS
        .iter()
        .map(|op| OperationView {
            name: op.name.to_string(),
            summary: words.fill(op.summary),
            kind: op.kind,
            changes: words.fill(op.changes),
            confirmation: op.confirmation,
            undo: op.undo,
            callers: op.callers.to_vec(),
            ready: op.ready,
            input_schema: op.input_schema(words),
            example: serde_json::from_str(op.example).unwrap_or(Value::Null),
        })
        .collect()
}

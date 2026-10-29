use tera::{Kwargs, Map, State, Value};

pub fn variables(args: Kwargs, _: &State) -> Value {
    Value::from(
        args.iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect::<Map>(),
    )
}

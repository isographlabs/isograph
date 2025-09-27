mod garbage_collection {
    mod basic_gc;
    mod inner_retained;
    mod multiple_calls;
    mod outer_retained;
    mod retained;
    mod retained_and_in_lru;
}

mod params {
    mod memo_ref_never_cloned;
    mod other_param_cloned_on_execute;
    mod source_id_never_cloned;
    mod with_serialize;
}

mod tracking_field {
    mod correctness;
    mod efficiency;
}

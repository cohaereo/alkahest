cbuffer cb_entid : register(b7) {
    uint entity_id;
}

uint PSMain() : SV_Target0 {
    if (entity_id == 0xFFFFFFFF) {
        discard;
    }
    return entity_id;
}
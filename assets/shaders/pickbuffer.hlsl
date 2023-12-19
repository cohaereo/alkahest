cbuffer cb0 : register(b0) {
    uint entity_id;
}

uint main() : SV_Target {
    return entity_id;
}
#include <metal_stdlib>
using namespace metal;

struct CrtInputComponent {
    uint64_t p;
    uint32_t two_e;
    uint32_t _padding;
    uint64_t hash1;
    uint64_t hash2;
};

inline uint64_t ualbf_bloom_get_index(uint64_t hash1, uint64_t hash2, uint64_t num_bits, uint32_t i) {
    uint64_t cur = hash1 + (uint64_t)i * hash2 + ((uint64_t)i * ((uint64_t)i - 1)) / 2;
    uint64_t max_bits = num_bits == 0 ? 1 : num_bits;
    return cur % max_bits;
}

kernel void crt_tensor_sieve(
    device const CrtInputComponent* inputs,
    device atomic_uint* bitmap,
    constant uint32_t& num_inputs,
    constant uint64_t& num_bits,
    constant uint32_t& num_hashes,
    uint id [[thread_position_in_grid]]
) {
    if (id >= num_inputs) return;

    uint64_t p = inputs[id].p;
    uint32_t two_e = inputs[id].two_e;
    uint64_t hash1 = inputs[id].hash1;
    uint64_t hash2 = inputs[id].hash2;

    bool obstructed = false;
    uint32_t moduli[4] = {3, 5, 7, 11};
    for (int m = 0; m < 4; m++) {
        uint32_t q = moduli[m];
        uint32_t sum = 0;
        uint32_t term = 1;
        uint32_t p_mod = (uint32_t)(p % q);
        for (uint32_t i = 0; i <= two_e; i++) {
            sum = (sum + term) % q;
            term = (term * p_mod) % q;
        }
        if (sum == 0) {
            obstructed = true;
            break;
        }
    }

    if (!obstructed) {
        for (uint32_t i = 0; i < num_hashes; i++) {
            uint64_t bit_idx = ualbf_bloom_get_index(hash1, hash2, num_bits, i);
            uint64_t word_idx = bit_idx / 32;
            uint32_t bit_mask = 1 << (bit_idx % 32);
            atomic_fetch_or_explicit(&bitmap[word_idx], bit_mask, memory_order_relaxed);
        }
    }
}

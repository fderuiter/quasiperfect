struct CrtInputComponent {
    unsigned long p;
    unsigned int two_e;
    unsigned int _padding;
    unsigned long hash1;
    unsigned long hash2;
};

inline unsigned long ualbf_bloom_get_index(unsigned long hash1, unsigned long hash2, unsigned long num_bits, unsigned int i) {
    unsigned long cur = hash1 + (unsigned long)i * hash2 + ((unsigned long)i * ((unsigned long)i - 1)) / 2;
    unsigned long max_bits = num_bits == 0 ? 1 : num_bits;
    return cur % max_bits;
}

__kernel void crt_tensor_sieve(
    __global const struct CrtInputComponent* inputs,
    __global volatile unsigned int* bitmap,
    const unsigned int num_inputs,
    const unsigned long num_bits,
    const unsigned int num_hashes
) {
    size_t id = get_global_id(0);
    if (id >= num_inputs) return;

    unsigned long p = inputs[id].p;
    unsigned int two_e = inputs[id].two_e;
    unsigned long hash1 = inputs[id].hash1;
    unsigned long hash2 = inputs[id].hash2;

    int obstructed = 0;
    unsigned int moduli[4] = {3, 5, 7, 11};
    for (int m = 0; m < 4; m++) {
        unsigned int q = moduli[m];
        unsigned int sum = 0;
        unsigned int term = 1;
        unsigned int p_mod = (unsigned int)(p % q);
        for (unsigned int i = 0; i <= two_e; i++) {
            sum = (sum + term) % q;
            term = (term * p_mod) % q;
        }
        if (sum == 0) {
            obstructed = 1;
            break;
        }
    }

    if (!obstructed) {
        for (unsigned int i = 0; i < num_hashes; i++) {
            unsigned long bit_idx = ualbf_bloom_get_index(hash1, hash2, num_bits, i);
            unsigned long word_idx = bit_idx / 32;
            unsigned int bit_mask = 1 << (bit_idx % 32);
            atomic_or(&bitmap[word_idx], bit_mask);
        }
    }
}

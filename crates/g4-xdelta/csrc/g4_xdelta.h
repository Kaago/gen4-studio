/* g4-xdelta: in-memory VCDIFF encode and decode over the vendored xdelta3 3.0.12.
 *
 * Return values: 0 = ok, *out_len bytes written to out;
 *                1 = out_cap too small, call again with a larger buffer;
 *                2 = failure, *msg points to a static NUL-terminated text.
 * Input lengths above UINT32_MAX fail with 2 (usize_t is uint32_t in 3.0.12). */
#ifndef G4_XDELTA_H
#define G4_XDELTA_H

#include <stddef.h>
#include <stdint.h>

int g4_xd3_encode(const uint8_t *source, size_t source_len, const uint8_t *target, size_t target_len,
                  const uint8_t *app_header, size_t app_header_len,
                  uint8_t *out, size_t out_cap, size_t *out_len, const char **msg);
int g4_xd3_decode(const uint8_t *source, size_t source_len, const uint8_t *patch, size_t patch_len,
                  uint8_t *out, size_t out_cap, size_t *out_len, const char **msg);

#endif

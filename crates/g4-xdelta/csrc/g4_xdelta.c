/* g4-xdelta shim (MIT), modelled on xd3_process_memory in xdelta3.c. The vendored
 * xdelta3.c (Apache-2.0, see NOTICE) is part of this translation unit, unchanged. */
#include "xdelta3.c"

#include "g4_xdelta.h"

/* The library's own messages are string literals, so they outlive the stream. */
static int g4_result(int ret, const xd3_stream *stream, const char **msg)
{
  if (ret == 0)
    return 0;
  if (ret == ENOSPC)
    return 1;
  if (stream->msg != NULL)
    *msg = stream->msg;
  else if (xd3_strerror(ret) != NULL)
    *msg = xd3_strerror(ret);
  else if (ret == ENOMEM)
    *msg = "out of memory";
  else
    *msg = "xdelta3 failed";
  return 2;
}

/* The whole source as one block, as xd3_process_memory does. */
static int g4_set_source(xd3_stream *stream, xd3_source *src, const uint8_t *source, usize_t source_len)
{
  memset(src, 0, sizeof(*src));
  src->blksize = source_len;
  src->onblk = source_len;
  src->curblk = source;
  src->curblkno = 0;
  src->max_winsize = source_len;
  return xd3_set_source_and_size(stream, src, source_len);
}

/* Above 1 GiB, xd3_set_source's power-of-two rounding of the source block overflows. */
#define G4_MAX_INPUT ((size_t) 1 << 30)
/* At or below 2 GiB, xd3_process_stream's uint32_t sum of written and pending bytes
 * cannot wrap, so its "insufficient output space" check holds. */
#define G4_MAX_OUTPUT ((size_t) 1 << 31)

static usize_t g4_cap(size_t out_cap)
{
  return out_cap > G4_MAX_OUTPUT ? (usize_t) G4_MAX_OUTPUT : (usize_t) out_cap;
}

int g4_xd3_encode(const uint8_t *source, size_t source_len, const uint8_t *target, size_t target_len,
                  const uint8_t *app_header, size_t app_header_len,
                  uint8_t *out, size_t out_cap, size_t *out_len, const char **msg)
{
  xd3_stream stream;
  xd3_config config;
  xd3_source src;
  usize_t written = 0;
  int ret;

  *out_len = 0;
  *msg = NULL;
  if (source_len > G4_MAX_INPUT || target_len > G4_MAX_INPUT || app_header_len > G4_MAX_INPUT)
    {
      *msg = "input larger than 1 GiB";
      return 2;
    }

  memset(&stream, 0, sizeof(stream));
  memset(&config, 0, sizeof(config));
  config.flags = XD3_ADLER32;
  config.winsize = xd3_min((usize_t) target_len, (usize_t) XD3_DEFAULT_WINSIZE);
  config.sprevsz = xd3_pow2_roundup(config.winsize);

  if ((ret = xd3_config_stream(&stream, &config)) != 0)
    goto exit;
  /* The encoder keeps the pointer; app_header outlives the stream. */
  if (app_header_len > 0)
    xd3_set_appheader(&stream, app_header, (usize_t) app_header_len);
  if ((ret = g4_set_source(&stream, &src, source, (usize_t) source_len)) != 0)
    goto exit;
  ret = xd3_process_stream(1, &stream, &xd3_encode_input, 1, target, (usize_t) target_len,
                           out, &written, g4_cap(out_cap));

exit:
  ret = g4_result(ret, &stream, msg);
  if (ret == 0)
    *out_len = written;
  xd3_free_stream(&stream);
  return ret;
}

int g4_xd3_decode(const uint8_t *source, size_t source_len, const uint8_t *patch, size_t patch_len,
                  uint8_t *out, size_t out_cap, size_t *out_len, const char **msg)
{
  xd3_stream stream;
  xd3_config config;
  xd3_source src;
  usize_t written = 0;
  int ret;

  *out_len = 0;
  *msg = NULL;
  if (source_len > G4_MAX_INPUT || patch_len > G4_MAX_INPUT)
    {
      *msg = "input larger than 1 GiB";
      return 2;
    }

  memset(&stream, 0, sizeof(stream));
  memset(&config, 0, sizeof(config));
  /* Flags 0: the decoder verifies Adler32 and skips the application header. */
  config.flags = 0;

  if ((ret = xd3_config_stream(&stream, &config)) != 0)
    goto exit;
  if ((ret = g4_set_source(&stream, &src, source, (usize_t) source_len)) != 0)
    goto exit;
  ret = xd3_process_stream(0, &stream, &xd3_decode_input, 1, patch, (usize_t) patch_len,
                           out, &written, g4_cap(out_cap));

exit:
  ret = g4_result(ret, &stream, msg);
  if (ret == 0)
    *out_len = written;
  xd3_free_stream(&stream);
  return ret;
}

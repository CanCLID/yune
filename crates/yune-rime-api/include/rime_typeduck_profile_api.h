#ifndef RIME_TYPEDUCK_PROFILE_API_H_
#define RIME_TYPEDUCK_PROFILE_API_H_

#include <rime_api.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct rime_typeduck_profile_api_t {
  RimeApi upstream;
  Bool (*config_list_append_bool)(RimeConfig* config,
                                  const char* key,
                                  Bool value);
  Bool (*config_list_append_int)(RimeConfig* config,
                                 const char* key,
                                 int value);
  Bool (*config_list_append_double)(RimeConfig* config,
                                    const char* key,
                                    double value);
  Bool (*config_list_append_string)(RimeConfig* config,
                                    const char* key,
                                    const char* value);
} RimeTypeDuckProfileApi;

RIME_API RimeTypeDuckProfileApi* rime_get_typeduck_profile_api(void);

#ifdef __cplusplus
}
#endif

#endif  // RIME_TYPEDUCK_PROFILE_API_H_

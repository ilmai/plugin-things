#ifndef FILTER_PLUGIN_H
#define FILTER_PLUGIN_H

#ifdef __cplusplus
extern "C" {
#endif    

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

const size_t PLINTH_AUV3_MAX_STRING_LENGTH = 100;

struct ParameterInfo {
    char*       identifier;
    char*       name;
    int64_t     parentGroupIndex;
    uint64_t    address;
    uint64_t    steps;
};

struct ParameterGroupInfo {
    char*       identifier;
    char*       name;
    int64_t     parentGroupIndex;
};

union AURenderEvent;

void* plinth_auv3_create();
void plinth_auv3_destroy(void* wrapper);

void plinth_auv3_activate(void* wrapper, double sampleRate, uint64_t maxBlockSize);
void plinth_auv3_deactivate(void* wrapper);

bool plinth_auv3_has_aux_bus();
double plinth_auv3_tail_length(void* wrapper);

void plinth_auv3_process(
    void* wrapper,
    const float** input,
    const float** aux,
    float **output,
    uint32_t channels,
    uint32_t frames,
    bool playing,
    double tempo,
    int64_t positionSamples,
    const union AURenderEvent *firstEvent
);

// Parameter interface
size_t plinth_auv3_group_count(void* wrapper);
void plinth_auv3_group_info(void* wrapper, size_t index, struct ParameterGroupInfo* info);

size_t plinth_auv3_parameter_count(void* wrapper);
void plinth_auv3_parameter_info(void* wrapper, size_t index, struct ParameterInfo* info);

float plinth_auv3_get_parameter_value(void* wrapper, uint64_t address);
void plinth_auv3_set_parameter_value(void* wrapper, uint64_t address, float value);

void plinth_auv3_parameter_normalized_to_string(void* wrapper, uint64_t address, float value, char* string);

// State interface
void plinth_auv3_load_state(void* wrapper, void* context, size_t (*read)(void*, uint8_t*, size_t));
void plinth_auv3_save_state(void* wrapper, void* context, size_t (*write)(void*, const uint8_t*, size_t));

// Editor interface
void plinth_auv3_preferred_editor_size(double* width, double* height);
void plinth_auv3_editor_set_scale(void* wrapper, double scale);
void plinth_auv3_editor_open(
    void* wrapper,
    void* parent,
    void* editor_context,
    void (*start_parameter_change)(void*, uint32_t),
    void (*change_parameter_value)(void*, uint32_t, float),
    void (*end_parameter_change)(void*, uint32_t),
    double scale
);
void plinth_auv3_editor_close(void* wrapper);

#ifdef __cplusplus
}
#endif

#endif // FILTER_PLUGIN_H

#ifndef DEMO_AUDIO_UNIT_H
#define DEMO_AUDIO_UNIT_H

#import <CoreAudioKit/CoreAudioKit.h>

AUAudioUnit* create_audio_unit(AudioComponentDescription componentDescription);

#endif // DEMO_AUDIO_UNIT_H

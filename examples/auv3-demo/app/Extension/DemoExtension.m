#import <CoreAudioKit/CoreAudioKit.h>
#import <Foundation/Foundation.h>

#import "Factory.h"

@interface DemoExtension : AUViewController<AUAudioUnitFactory>
@end

@implementation DemoExtension
- (AUAudioUnit *)createAudioUnitWithComponentDescription:(AudioComponentDescription)desc error:(NSError *__autoreleasing _Nullable *)error
{
    return create_audio_unit();
}
@end

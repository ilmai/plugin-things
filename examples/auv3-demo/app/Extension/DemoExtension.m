#import <CoreAudioKit/CoreAudioKit.h>
#import <Foundation/Foundation.h>

@interface DemoExtension : AUViewController<AUAudioUnitFactory>
@end

@implementation DemoExtension
- (AUAudioUnit *)createAudioUnitWithComponentDescription:(AudioComponentDescription)desc error:(NSError *__autoreleasing _Nullable *)error
{
    return NULL;
}
@end

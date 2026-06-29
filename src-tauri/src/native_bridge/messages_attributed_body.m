#import <Foundation/Foundation.h>
#include <stddef.h>
#include <string.h>

enum {
    MorrowAttributedBodyDecodeFailed = 1,
    MorrowAttributedBodyTextTruncated = 2,
};

typedef struct {
    int ok;
    int error_code;
    char text[4096];
    char message[256];
} MorrowMessagesAttributedBodyResult;

static BOOL SetString(char *buffer, size_t capacity, NSString *value) {
    if (capacity == 0) {
        return NO;
    }
    buffer[0] = '\0';
    if (value == nil) {
        return YES;
    }
    const char *utf8 = value.UTF8String;
    if (utf8 == NULL) {
        return NO;
    }
    size_t length = strlen(utf8);
    size_t copyLength = length < capacity ? length : capacity - 1;
    memcpy(buffer, utf8, copyLength);
    buffer[copyLength] = '\0';
    return length < capacity;
}

static void SetError(MorrowMessagesAttributedBodyResult *result, int code, NSString *message) {
    result->ok = 0;
    result->error_code = code;
    SetString(result->message, sizeof(result->message), message);
}

static id LegacyUnarchiveObject(NSData *data) {
    Class unarchiver = NSClassFromString(@"NSUnarchiver");
    SEL selector = NSSelectorFromString(@"unarchiveObjectWithData:");
    if (unarchiver == Nil || ![unarchiver respondsToSelector:selector]) {
        return nil;
    }
    NSMethodSignature *signature = [unarchiver methodSignatureForSelector:selector];
    if (signature == nil) {
        return nil;
    }
    NSInvocation *invocation = [NSInvocation invocationWithMethodSignature:signature];
    invocation.target = unarchiver;
    invocation.selector = selector;
    [invocation setArgument:&data atIndex:2];
    [invocation invoke];
    __unsafe_unretained id object = nil;
    [invocation getReturnValue:&object];
    return object;
}

void morrow_messages_attributed_body_text(
    const unsigned char *bytes,
    size_t length,
    MorrowMessagesAttributedBodyResult *result
) {
    if (result == NULL) {
        return;
    }
    memset(result, 0, sizeof(*result));
    if (bytes == NULL || length == 0) {
        SetError(result, MorrowAttributedBodyDecodeFailed, @"attributed body is empty");
        return;
    }

    NSData *data = [NSData dataWithBytes:bytes length:length];
    id object = nil;
    @try {
        NSError *error = nil;
        object = [NSKeyedUnarchiver unarchiveTopLevelObjectWithData:data error:&error];
        if (error != nil) {
            object = LegacyUnarchiveObject(data);
        }
    } @catch (NSException *exception) {
        SetError(result, MorrowAttributedBodyDecodeFailed, @"attributed body unarchive failed");
        return;
    }

    NSString *text = nil;
    if ([object isKindOfClass:[NSAttributedString class]]) {
        text = [(NSAttributedString *)object string];
    } else if ([object isKindOfClass:[NSString class]]) {
        text = (NSString *)object;
    }
    if (text == nil || text.length == 0) {
        SetError(result, MorrowAttributedBodyDecodeFailed, @"attributed body did not contain text");
        return;
    }
    if (!SetString(result->text, sizeof(result->text), text)) {
        SetError(result, MorrowAttributedBodyTextTruncated, @"attributed body text was too long");
        return;
    }
    result->ok = 1;
}

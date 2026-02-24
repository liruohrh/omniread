import 'package:flutter_test/flutter_test.dart';

Matcher throwsandLogA<T>({String message = ""}) =>
    _ThrowsAndLogMatcher<T>(message: message);

class _ThrowsAndLogMatcher<T> extends Matcher {
  final String message;
  const _ThrowsAndLogMatcher({this.message = ""});

  @override
  Description describe(Description description) {
    return description.add('throws an exception of type $T');
  }

  @override
  bool matches(dynamic item, Map matchState) {
    if (item is! Future) {
      return false;
    }
    item.then(
      (value) {
        matchState['returnValue'] = value;
      },
      onError: (e, stackTrace) {
        matchState['exception'] = e;
        matchState['stackTrace'] = stackTrace;
      },
    );

    return true;
  }

  @override
  Description describeMismatch(
    dynamic item,
    Description mismatchDescription,
    Map matchState,
    bool verbose,
  ) {
    if (item is! Future) {
      return mismatchDescription.add('is not a Future');
    }

    final exception = matchState['exception'];

    if (exception == null) {
      final returnValue = matchState['returnValue'];
      return mismatchDescription.add('did not throw, returned: `$returnValue`');
    }

    final stackTrace = matchState['stackTrace'];
    print('throwsA<$T> caught: $exception\n$stackTrace\n====================');

    if (exception is! T) {
      return mismatchDescription
          .add('threw ${exception.runtimeType} instead of $T');
    }

    if (message.isNotEmpty && !exception.toString().contains(message)) {
      return mismatchDescription.add(
          'threw ${exception.runtimeType} without the expected message=$message but got ${exception.toString()}');
    }

    return mismatchDescription;
  }
}

import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:omniread/services/headless_renderer.dart';
import 'package:omniread/services/render_pool.dart';

/// A mock render function that completes after [delay] with '<html>{url}</html>'.
/// If [shouldFail] is true, throws an exception instead.
RenderFn mockRenderFn({
  Duration delay = const Duration(milliseconds: 50),
  bool shouldFail = false,
}) {
  return (String url, String jsCode) async {
    await Future.delayed(delay);
    if (shouldFail) throw Exception('Mock render failed: $url');
    return '<html>$url</html>';
  };
}

/// A render function that never completes (hangs forever), for testing cancel.
RenderFn hangingRenderFn() {
  return (String url, String jsCode) => Completer<String>().future;
}

/// A render function that tracks invocation count and in-flight count.
class TrackedRenderFn {
  int started = 0;
  int finished = 0;
  int maxConcurrentObserved = 0;
  int _inFlight = 0;
  final Duration delay;
  final bool shouldFail;

  TrackedRenderFn({
    this.delay = const Duration(milliseconds: 50),
    this.shouldFail = false,
  });

  Future<String> call(String url, String jsCode) async {
    started++;
    _inFlight++;
    if (_inFlight > maxConcurrentObserved) {
      maxConcurrentObserved = _inFlight;
    }
    try {
      await Future.delayed(delay);
      if (shouldFail) throw Exception('Mock render failed: $url');
      return '<html>$url</html>';
    } finally {
      _inFlight--;
      finished++;
    }
  }
}

void main() {
  group('RenderPool', () {
    test('basic: submit and get result', () async {
      final pool = RenderPool(maxConcurrent: 2, renderFn: mockRenderFn());
      final task = pool.submit('https://a.com', '');
      final html = await task.future;
      expect(html, '<html>https://a.com</html>');
      expect(task.isPending, false);
      expect(task.isCancelled, false);
      pool.dispose();
    });

    test('concurrency limit is respected', () async {
      final tracker = TrackedRenderFn(delay: const Duration(milliseconds: 100));
      final pool = RenderPool(maxConcurrent: 2, renderFn: tracker.call);

      final tasks = <RenderTask>[];
      for (var i = 0; i < 5; i++) {
        tasks.add(pool.submit('https://$i.com', ''));
      }

      // Give some time for tasks to start.
      await Future.delayed(const Duration(milliseconds: 20));

      // At most 2 should be running at once.
      expect(pool.runningCount, 2);
      expect(pool.queuedCount, 3);

      // Wait for all to finish.
      await Future.wait(tasks.map((t) => t.future));
      expect(tracker.maxConcurrentObserved, 2);
      expect(tracker.finished, 5);
      pool.dispose();
    });

    test('cancel a queued task', () async {
      final pool = RenderPool(
        maxConcurrent: 1,
        renderFn: mockRenderFn(delay: const Duration(milliseconds: 200)),
      );

      final task1 = pool.submit('https://a.com', '');
      final task2 = pool.submit('https://b.com', ''); // queued

      // Cancel the queued task immediately.
      task2.cancel();
      expect(task2.isCancelled, true);

      await expectLater(task2.future, throwsA(isA<RenderCancelledException>()));
      await task1.future; // first should still complete fine
      pool.dispose();
    });

    test('cancel a running task', () async {
      final pool = RenderPool(
        maxConcurrent: 1,
        renderFn: hangingRenderFn(),
      );

      final task = pool.submit('https://a.com', '');
      await Future.delayed(const Duration(milliseconds: 10));
      expect(pool.runningCount, 1);

      task.cancel();
      await expectLater(task.future, throwsA(isA<RenderCancelledException>()));
      pool.dispose();
    });

    test('cancelAll cancels queued and running tasks', () async {
      final pool = RenderPool(
        maxConcurrent: 1,
        renderFn: hangingRenderFn(),
      );

      final task1 = pool.submit('https://a.com', ''); // running (hangs)
      final task2 = pool.submit('https://b.com', ''); // queued
      final task3 = pool.submit('https://c.com', ''); // queued

      // Attach error listeners BEFORE cancel, so errors don't go unhandled.
      final f1 = task1.future.then((_) => true, onError: (_) => false);
      final f2 = task2.future.then((_) => true, onError: (_) => false);
      final f3 = task3.future.then((_) => true, onError: (_) => false);

      await Future.delayed(const Duration(milliseconds: 10));
      pool.cancelAll();

      await f1;
      await f2;
      await f3;
      expect(task1.isCancelled, true);
      expect(task2.isCancelled, true);
      expect(task3.isCancelled, true);
      pool.dispose();
    });

    test('dispose rejects new submissions', () async {
      final pool = RenderPool(maxConcurrent: 2, renderFn: mockRenderFn());
      pool.dispose();
      expect(() => pool.submit('https://a.com', ''), throwsStateError);
    });

    test('render error propagates to task future', () async {
      final pool = RenderPool(
        maxConcurrent: 2,
        renderFn: mockRenderFn(shouldFail: true),
      );
      final task = pool.submit('https://a.com', '');
      await expectLater(task.future, throwsA(isA<Exception>()));
      pool.dispose();
    });

    test('dynamic maxConcurrent increase drains queue', () async {
      final tracker = TrackedRenderFn(delay: const Duration(milliseconds: 100));
      final pool = RenderPool(maxConcurrent: 1, renderFn: tracker.call);

      for (var i = 0; i < 4; i++) {
        pool.submit('https://$i.com', '');
      }
      await Future.delayed(const Duration(milliseconds: 10));
      expect(pool.runningCount, 1);
      expect(pool.queuedCount, 3);

      // Increase concurrency.
      pool.maxConcurrent = 3;
      await Future.delayed(const Duration(milliseconds: 10));
      // Now up to 3 should be running.
      expect(pool.runningCount, 3);
      expect(pool.queuedCount, 1);

      // Wait for completion.
      await Future.delayed(const Duration(milliseconds: 300));
      expect(tracker.finished, 4);
      pool.dispose();
    });

    test('cancel already-completed task is no-op', () async {
      final pool = RenderPool(maxConcurrent: 2, renderFn: mockRenderFn());
      final task = pool.submit('https://a.com', '');
      final html = await task.future;
      expect(html, contains('a.com'));

      // Should not throw.
      task.cancel();
      expect(task.isCancelled, false); // Already completed, cancel is no-op.
      pool.dispose();
    });

    test('queued count decreases as tasks run', () async {
      final tracker = TrackedRenderFn(delay: const Duration(milliseconds: 50));
      final pool = RenderPool(maxConcurrent: 1, renderFn: tracker.call);

      final tasks = <RenderTask>[];
      for (var i = 0; i < 3; i++) {
        tasks.add(pool.submit('https://$i.com', ''));
      }

      await Future.delayed(const Duration(milliseconds: 10));
      expect(pool.queuedCount, 2);

      await Future.delayed(const Duration(milliseconds: 60));
      expect(pool.queuedCount, lessThanOrEqualTo(1));

      await Future.wait(tasks.map((t) => t.future));
      expect(pool.queuedCount, 0);
      expect(pool.runningCount, 0);
      pool.dispose();
    });
  });
}

import 'dart:async';

import 'package:flutter_inappwebview/flutter_inappwebview.dart';

/// Renders a URL in a headless (invisible) system WebView, executes JavaScript
/// to wait for elements/conditions, then captures and returns the rendered HTML.
///
/// ## Cancel/Timeout Design
/// Cancel and timeout are managed by the **caller**, not by this class.
/// Both user-exit and timeout use the same unified mechanism:
///   - User exits: caller calls `cancel()` in widget's `dispose()`
///   - Timeout:    caller uses `Future.timeout()` then calls `cancel()`
/// This keeps the renderer focused on rendering, and the caller controls lifecycle.
///
/// Example usage:
/// ```dart
/// final renderer = HeadlessRenderer();
/// try {
///   final html = await renderer
///       .render('https://example.com', 'await new Promise(r => setTimeout(r, 1000))')
///       .timeout(Duration(seconds: 30), onTimeout: () {
///     renderer.cancel();
///     throw TimeoutException('Render timed out');
///   });
/// } catch (e) {
///   renderer.cancel(); // ensure cleanup
/// }
/// ```
class HeadlessRenderer {
  HeadlessInAppWebView? _webView;
  Completer<String>? _completer;
  bool _isCancelled = false;

  /// Whether a render task is currently running.
  bool get isRendering => _completer != null && !_completer!.isCompleted;

  /// Render [url] in a headless WebView, execute [jsCode] after page loads,
  /// then return the fully rendered HTML (document.documentElement.outerHTML).
  ///
  /// [jsCode] is executed via `callAsyncJavaScript` so async/await is supported.
  /// It has no return value — it just waits for elements/conditions to appear.
  /// After [jsCode] completes, the full page HTML is captured and returned.
  Future<String> render(String url, String jsCode) async {
    if (isRendering) {
      throw StateError(
          'A render task is already running. Call cancel() first.');
    }

    _isCancelled = false;
    _completer = Completer<String>();

    _webView = HeadlessInAppWebView(
      initialUrlRequest: URLRequest(url: WebUri(url)),
      onLoadStop: (controller, loadedUrl) async {
        if (_isCancelled) return;

        try {
          // Execute the JS wait condition. callAsyncJavaScript supports async/await.
          // The JS code has no return value — it just waits for rendering to complete.
          if (jsCode.isNotEmpty) {
            if (_isCancelled) return;
            await controller.callAsyncJavaScript(functionBody: jsCode);
          }

          if (_isCancelled) return;

          // Capture the fully rendered HTML after JS wait completes.
          final html = await controller.evaluateJavascript(
            source: 'document.documentElement.outerHTML',
          );

          if (!_completer!.isCompleted) {
            _completer!.complete(html is String ? html : html.toString());
          }
        } catch (e) {
          if (!_completer!.isCompleted) {
            _completer!.completeError(e);
          }
        } finally {
          _dispose();
        }
      },
      onReceivedError: (controller, request, error) {
        if (!_completer!.isCompleted) {
          _completer!.completeError(
            Exception(
                'Page load error (${error.type}): ${error.description} [url=${request.url}]'),
          );
        }
        _dispose();
      },
      onReceivedHttpError: (controller, request, response) {
        if (!_completer!.isCompleted) {
          _completer!.completeError(
            Exception(
                'HTTP error (${response.statusCode}): ${response.reasonPhrase} [url=${request.url}]'),
          );
        }
        _dispose();
      },
    );

    await _webView!.run();
    return _completer!.future;
  }

  /// Cancel the current render task.
  ///
  /// This is the unified cancel mechanism used by both scenarios:
  /// - User exits: widget.dispose() -> cancel()
  /// - Timeout:    Future.timeout(duration) -> cancel()
  ///
  /// Safe to call multiple times or when no task is running.
  void cancel() {
    _isCancelled = true;
    if (_completer != null && !_completer!.isCompleted) {
      _completer!.completeError(Exception('Rendering cancelled'));
    }
    _dispose();
  }

  void _dispose() {
    _webView?.dispose();
    _webView = null;
  }
}

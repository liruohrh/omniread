import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_inappwebview/flutter_inappwebview.dart';

/// Base interface for all render-related exceptions.
/// Users can catch [RenderException] to handle all render errors uniformly.
abstract class RenderException implements Exception {
  String get url;
}

/// Thrown when a render operation is cancelled.
class RenderCancelledException implements RenderException {
  @override
  final String url;
  const RenderCancelledException(this.url);

  @override
  String toString() => 'RenderCancelledException: $url';
}

/// Thrown when a page fails to load (network error, DNS failure, etc.).
class RenderLoadException implements RenderException {
  @override
  final String url;
  final String message;
  const RenderLoadException(this.url, this.message);

  @override
  String toString() => 'RenderLoadException($url): $message';
}

/// Thrown when the server responds with an HTTP error status code.
class RenderHttpException implements RenderException {
  @override
  final String url;
  final int statusCode;
  final String? reasonPhrase;
  const RenderHttpException(this.url, this.statusCode, this.reasonPhrase);

  @override
  String toString() =>
      'RenderHttpException($url): $statusCode ${reasonPhrase ?? ''}';
}

/// Thrown when JavaScript execution fails.
class RenderJsException implements RenderException {
  @override
  final String url;
  final Object cause;
  final String message;
  const RenderJsException(this.url, this.cause, {this.message = ""});
  @override
  String toString() =>
      'RenderJsException(${message.isNotEmpty ? "$message, " : ""}$url): $cause';
}

/// Thrown when attempting to render on an already cancelled renderer.
class RenderAlreadyCancelledException implements RenderException {
  @override
  final String url;
  const RenderAlreadyCancelledException(this.url);

  @override
  String toString() => 'RenderAlreadyCancelledException: $url';
}

/// Thrown when attempting to render multiple times on the same renderer.
class RenderAlreadyRunningException implements RenderException {
  @override
  final String url;
  const RenderAlreadyRunningException(this.url);

  @override
  String toString() => 'RenderAlreadyRunningException: $url';
}

/// Renders a URL in a headless (invisible) system WebView, executes JavaScript
/// to wait for elements/conditions, then captures and returns the rendered HTML.
///
/// ## Usage
/// Each `HtmlRenderer` instance is bound to a single URL and jsCode.
/// Create a new instance for each render task.
///
/// Example usage:
/// ```dart
/// final renderer = HtmlRenderer(
///   'https://example.com',
///   'await new Promise(r => setTimeout(r, 1000))',
/// );
/// try {
///   final html = await renderer
///       .render()
///       .timeout(Duration(seconds: 30), onTimeout: () {
///     renderer.cancel();
///     throw TimeoutException('Render timed out');
///   });
/// } catch (e) {
///   renderer.cancel(); // ensure cleanup
/// }
/// ```
class HtmlRenderer {
  final String url;
  final String jsCode;

  HeadlessInAppWebView? _webView;
  Completer<String>? _completer;
  bool _isCancelled = false;
  bool _isDisposing = false;
  bool _hasRendered = false;

  /// Creates a renderer for the given [url] and [jsCode].
  /// Each instance can only be used once.
  HtmlRenderer(this.url, this.jsCode);

  /// Whether a render task is currently running.
  bool get isRendering => _completer != null && !_completer!.isCompleted;

  /// Whether this renderer has been cancelled.
  bool get isCancelled => _isCancelled;

  /// Whether this renderer has already completed a render.
  bool get hasRendered => _hasRendered;

  /// Render [url] in a headless WebView, execute [jsCode] after page loads,
  /// then return the fully rendered HTML (document.documentElement.outerHTML).
  ///
  /// [jsCode] is executed via `callAsyncJavaScript` so async/await is supported.
  /// It has no return value — it just waits for elements/conditions to appear.
  /// After [jsCode] completes, the full page HTML is captured and returned.
  ///
  /// Throws [RenderAlreadyCancelledException] if this renderer has been cancelled.
  /// Throws [RenderAlreadyRunningException] if render is already in progress.
  /// Throws [StateError] if render has already completed.
  Future<String> render() async {
    if (_isCancelled) {
      throw RenderAlreadyCancelledException(url);
    }

    if (isRendering) {
      throw RenderAlreadyRunningException(url);
    }

    if (_hasRendered) {
      throw StateError(
          'Render has already completed. Create a new HtmlRenderer instance.');
    }

    _isDisposing = false;
    _completer = Completer<String>();

    _webView = HeadlessInAppWebView(
      initialUrlRequest: URLRequest(url: WebUri(url)),
      onLoadStop: (controller, loadedUrl) async {
        if (_isCancelled) return;

        try {
          debugPrint('HtmlRenderer: loaded $url');
          if (jsCode.isNotEmpty) {
            if (_isCancelled) return;

            // check js syntax
            //  for webview do not throw error for invalid syntax
            //. ensure platforms: android
            try {
              final jsResult = await controller.callAsyncJavaScript(
                  functionBody: "new Function(jsCode);",
                  arguments: {"jsCode": "async ()=>{ $jsCode }"});
              if (jsResult?.error != null) {
                throw jsResult!.error!;
              }
            } catch (e) {
              throw RenderJsException(url, e, message: "check js syntax");
            }
            // invoke
            final jsResult = await controller.callAsyncJavaScript(
              functionBody: jsCode,
            );
            if (jsResult?.error != null) {
              throw jsResult!.error!;
            }
          }

          if (_isCancelled) return;

          // Capture the fully rendered HTML after JS wait completes.
          final html = await controller.evaluateJavascript(
            source: 'document.documentElement.outerHTML',
          );

          if (!_completer!.isCompleted) {
            _hasRendered = true;
            _completer!.complete(html is String ? html : html.toString());
          }
        } catch (e) {
          if (!_completer!.isCompleted) {
            _completer!.completeError(RenderJsException(url, e));
          }
        }
      },
      onReceivedError: (controller, request, error) {
        if (!_completer!.isCompleted) {
          _completer!.completeError(
            RenderLoadException(url, '${error.type}: ${error.description}'),
          );
        }
      },
      onReceivedHttpError: (controller, request, response) {
        final requestUrl = request.url.toString();
        if (requestUrl == url && !_completer!.isCompleted) {
          _completer!.completeError(
            RenderHttpException(
                url, response.statusCode ?? 0, response.reasonPhrase),
          );
        } else {
          debugPrint(
              'HtmlRenderer: HTTP error for $requestUrl, status=${response.statusCode}');
        }
      },
    );

    try {
      await _webView!.run();
      final result = await _completer!.future;
      return result;
    } finally {
      _dispose();
    }
  }

  /// Cancel the current render task.
  ///
  /// This is the unified cancel mechanism used by both scenarios:
  /// - User exits: widget.dispose() -> cancel()
  /// - Timeout:    Future.timeout(duration) -> cancel()
  ///
  /// Safe to call multiple times or when no task is running.
  /// Once cancelled, this renderer cannot be used again.
  void cancel() {
    _isCancelled = true;
    if (_completer != null && !_completer!.isCompleted) {
      _completer!.completeError(RenderCancelledException(url));
    }
    _dispose();
  }

  void _dispose() {
    if (_isDisposing || _webView == null) return;
    _isDisposing = true;
    Future.delayed(const Duration(milliseconds: 500), () {
      try {
        _webView?.dispose();
      } finally {
        _isDisposing = false;
        _webView = null;
      }
    });
  }
}

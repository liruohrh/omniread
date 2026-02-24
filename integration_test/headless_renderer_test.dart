import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:omniread/services/headless_renderer.dart';

import 'helper.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  group('HtmlRenderer', () {
    test('renders a simple page and returns HTML', () async {
      final renderer = HtmlRenderer('https://example.com', '');
      final html = await renderer.render();
      expect(html, contains('<html'));
      expect(html, contains('Example Domain'));
    });

    test('executes JS wait code before capturing HTML', () async {
      final renderer = HtmlRenderer(
        'https://example.com',
        'await new Promise(r => setTimeout(r, 500));',
      );
      final html = await renderer.render();
      expect(html, contains('<html'));
    });

    test('cancel stops a running render', () async {
      final renderer = HtmlRenderer(
        'https://example.com',
        'await new Promise(r => setTimeout(r, 30000));', // hang for 30s
      );
      final future = renderer.render().catchError((e) {
        expect(e is RenderCancelledException, isTrue);
        return ""; // 返回一个默认值，让 future 正常完成
      });
      await Future.delayed(const Duration(milliseconds: 500));

      renderer.cancel();
      await future; // 等待设置了错误处理器的 future
    });

    test('JS syntax error throws RenderJsException', () async {
      final renderer = HtmlRenderer(
        'https://example.com',
        'this is not valid javascript !!!{{{',
      );
      final future = renderer.render();
      expect(future, throwsandLogA<RenderJsException>());
    });

    test('JS runtime exception throws RenderJsException', () async {
      final renderer = HtmlRenderer(
        'https://example.com',
        'throw new Error("intentional test error");',
      );
      final future = renderer.render();
      expect(future, throwsandLogA<RenderJsException>());
    });

    test('render after cancelled throws RenderAlreadyCancelledException',
        () async {
      final renderer = HtmlRenderer(
        'https://example.com',
        'await new Promise(r => setTimeout(r, 1000));',
      );
      renderer.cancel();
      expect(
          renderer.render(), throwsA(isA<RenderAlreadyCancelledException>()));
    });

    test('double render throws RenderAlreadyRunningException', () async {
      final renderer = HtmlRenderer(
        'https://example.com',
        'await new Promise(r => setTimeout(r, 30000));',
      );
      // Start first render (don't await)
      renderer.render().catchError((_) => '');
      await Future.delayed(const Duration(milliseconds: 100));
      // Second render should throw
      expect(renderer.render(), throwsA(isA<RenderAlreadyRunningException>()));
      renderer.cancel();
    });
  });
}

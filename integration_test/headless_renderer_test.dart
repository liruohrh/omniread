import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:omniread/services/headless_renderer.dart';

import 'helper.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  group('HeadlessRenderer', () {
    test('renders a simple page and returns HTML', () async {
      final renderer = HeadlessRenderer();
      final html = await renderer.render('https://example.com', '');
      expect(html, contains('<html'));
      expect(html, contains('Example Domain'));
    });

    test('executes JS wait code before capturing HTML', () async {
      final renderer = HeadlessRenderer();
      final html = await renderer.render(
        'https://example.com',
        'await new Promise(r => setTimeout(r, 500));',
      );
      expect(html, contains('<html'));
    });

    test('cancel stops a running render', () async {
      final renderer = HeadlessRenderer();
      final future = renderer
          .render(
        'https://example.com',
        'await new Promise(r => setTimeout(r, 30000));', // hang for 30s
      )
          .catchError((e) {
        expect(e is RenderCancelledException, isTrue);
        return ""; // 返回一个默认值，让 future 正常完成
      });
      await Future.delayed(const Duration(milliseconds: 500));

      renderer.cancel();
      await future; // 等待设置了错误处理器的 future
    });

    test('JS syntax error throws RenderJsException', () async {
      final renderer = HeadlessRenderer();
      final future = renderer.render(
        'https://example.com',
        'this is not valid javascript !!!{{{',
      );
      expect(future, throwsandLogA<RenderJsException>());
    });

    test('JS runtime exception throws RenderJsException', () async {
      final renderer = HeadlessRenderer();
      final future = renderer.render(
        'https://example.com',
        'throw new Error("intentional test error");',
      );
      expect(future, throwsandLogA<RenderJsException>());
    });
  });
}

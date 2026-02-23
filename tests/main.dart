import 'package:flutter/material.dart';
import 'package:omniread/gen/rust/frb_generated.dart';
import 'headless_render_test.dart';
import 'pool_render_test.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  runApp(const TestApp());
}

class TestApp extends StatelessWidget {
  const TestApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'OmniRead Tests',
      theme: ThemeData(
        colorSchemeSeed: Colors.blue,
        useMaterial3: true,
      ),
      home: const TestHomePage(),
    );
  }
}

class TestHomePage extends StatelessWidget {
  const TestHomePage({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('OmniRead Tests')),
      body: ListView(
        children: [
          _TestCategory(
            title: 'WebView',
            icon: Icons.web,
            items: [
              _TestItem(
                title: 'Single Render',
                subtitle: '单次 HeadlessWebView 渲染测试',
                builder: (context) => const HeadlessRenderTestPage(),
              ),
              _TestItem(
                title: 'Pool (Concurrent)',
                subtitle: 'RenderPool 并发渲染测试',
                builder: (context) => const PoolRenderTestPage(),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _TestCategory extends StatelessWidget {
  final String title;
  final IconData icon;
  final List<_TestItem> items;

  const _TestCategory({
    required this.title,
    required this.icon,
    required this.items,
  });

  @override
  Widget build(BuildContext context) {
    return ExpansionTile(
      leading: Icon(icon),
      title: Text(title),
      initiallyExpanded: true,
      children: items,
    );
  }
}

class _TestItem extends StatelessWidget {
  final String title;
  final String subtitle;
  final WidgetBuilder builder;

  const _TestItem({
    required this.title,
    required this.subtitle,
    required this.builder,
  });

  @override
  Widget build(BuildContext context) {
    return ListTile(
      contentPadding: const EdgeInsets.only(left: 56, right: 16),
      title: Text(title),
      subtitle: Text(subtitle),
      trailing: const Icon(Icons.chevron_right),
      onTap: () {
        Navigator.of(context).push(
          MaterialPageRoute(builder: builder),
        );
      },
    );
  }
}

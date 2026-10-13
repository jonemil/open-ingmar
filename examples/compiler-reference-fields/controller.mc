using Toybox.WatchUi;
class ViewController extends WatchUi.View {
    var target as HelloView;
    function initialize(view as HelloView) {
        View.initialize();
        self.target = view;
    }
    function update(amount) { return self.target.add(amount); }
    function view() as HelloView { return self.target; }
    function current() { return self.target.current(); }
}

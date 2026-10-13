module Views {
    function make() as HelloView { return new HelloView(); }
    function pass(view as HelloView) as HelloView { return view; }
    function update(view as HelloView, amount) { return view.add(amount); }
}

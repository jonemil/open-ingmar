import Toybox.Lang;

typedef Shared as Types.Value;
typedef OptionalToken as Token or Null;

module Types {
    typedef Value as Number or String;
    typedef Element as Number;
    typedef Values as Array<Element>;
    typedef Mapping as Dictionary<String, Element>;

    function array(values as Values) as Values { return values; }
    function dictionary(values as Mapping) as Mapping { return values; }
}

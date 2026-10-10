function choose(value) {
    var answer = 40;
    var message = "start";
    System.println(message);
    if (value == 0) {
        answer = answer + 3;
        message = "zero";
    } else {
        answer = answer + 7;
        message = "one";
    }
    System.println(message);
    System.println(answer);
    answer = answer + 1;
    return answer;
}

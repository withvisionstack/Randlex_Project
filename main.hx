const CHILD_AGE: int64 = 12;
const ADULT_AGE: int64 = 18;
const SENIOR_AGE: int64 = 60;

function CheckAge($age: int64): str {

    if ($age < CHILD_AGE) {
        return "Crianca";
    }

    elseif ($age < ADULT_AGE) {
        return "Adolescente";
    }

    elseif ($age < SENIOR_AGE) {
        return "Adulto";
    }

    else {
        return "Idoso";
    }

};

$myAge: int64 = 25;

Show(CheckAge($myAge));


package tests

import (
	"testing"
)

type A struct {
	A int
}

type B struct {
	A struct {
		C string
	}
	B int
}

func Test_X(t *testing.T) {
	m := map[string]int{"a": 10}
	i := 20
	ok := false
	if i, ok = m["ab"]; !ok {
		println(i)
	} else {
		println(i)
	}

}

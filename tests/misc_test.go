package tests

import (
	ty "go/types"
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

func (A) UnmarshalJSON(b []byte) error {
	return nil
}

func (B) UnmarshalJSON(b []byte) error {
	return nil
}

type Object interface {
	UnmarshalJSON(b []byte) error
}

func (o Object) UnmarshalJSON(b []byte) error {
	*o = B{}
	ty.Checker{}
	return nil
}

func Test_X(t *testing.T) {

}

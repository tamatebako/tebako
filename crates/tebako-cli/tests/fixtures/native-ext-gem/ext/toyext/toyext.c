#include <ruby.h>

static VALUE
toyext_answer(VALUE self)
{
    return INT2FIX(42);
}

void
Init_toyext(void)
{
    VALUE mod = rb_define_module("Toyext");
    rb_define_singleton_method(mod, "answer", toyext_answer, 0);
}

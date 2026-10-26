// Standalone adapter for the pinned engine's float math; formulas are unchanged.
#pragma once
typedef float Simple;
typedef int int32_t;
typedef unsigned int u_int32_t;
#define GET_FLOAT_WORD(i,d) do { union {float f; unsigned int u;} q; q.f=(d); (i)=q.u; } while(0)
#define SET_FLOAT_WORD(d,i) do { union {float f; unsigned int u;} q; q.u=(i); (d)=q.f; } while(0)
namespace streflop_libm {
inline bool isinf(float x) {return __builtin_isinf(x);}
inline float fabsf(float x) {return __builtin_fabsf(x);}
inline float __copysignf(float x,float y) {return __builtin_copysignf(x,y);}
inline float __ieee754_sqrtf(float x) {return __builtin_sqrtf(x);}
inline float __floorf(float x) {return __builtin_floorf(x);}
float __scalbnf(float,int);
extern "C" float cabal_math_powf(float,float);
extern "C" float cabal_math_acosf(float);
float __kernel_sinf(float,float,int);
float __kernel_cosf(float,float);
extern "C" float cabal_math_sinf(float);
extern "C" float cabal_math_cosf(float);
int __ieee754_rem_pio2f(float,float*);
int __kernel_rem_pio2f(float*,float*,int,int,int,const int*);
}

static_assert(sizeof(float) == 4 && sizeof(int) == 4, "32-bit math types required");

def wrap_generator(generator, wrapper_function):
    """
    Calls each item in the generator with the function provided.
    """
    for item in generator:
        yield wrapper_function(item)

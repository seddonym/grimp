from grimp import exceptions


class TestSourceSyntaxError:
    def test_str(self):
        assert "Syntax error in path/to/somefile.py, line 3: something wrong" == str(
            exceptions.SourceSyntaxError(
                filename="path/to/somefile.py", lineno=3, text="something wrong",
            )
        )

    def test_same_values_are_equal(self):
        assert exceptions.SourceSyntaxError(
            filename="path/to/somefile.py", lineno=3, text="something wrong",
        ) == exceptions.SourceSyntaxError(
            filename="path/to/somefile.py", lineno=3, text="something wrong",
        )

    def test_different_filenames_are_not_equal(self):
        assert exceptions.SourceSyntaxError(
            filename="path/to/somefile.py", lineno=3, text="something wrong",
        ) != exceptions.SourceSyntaxError(
            filename="path/to/anotherfile.py", lineno=3, text="something wrong",
        )

    def test_different_linenos_are_not_equal(self):
        assert exceptions.SourceSyntaxError(
            filename="path/to/somefile.py", lineno=3, text="something wrong",
        ) != exceptions.SourceSyntaxError(
            filename="path/to/somefile.py", lineno=4, text="something wrong",
        )

    def test_different_texts_are_not_equal(self):
        assert exceptions.SourceSyntaxError(
            filename="path/to/somefile.py", lineno=3, text="something wrong",
        ) != exceptions.SourceSyntaxError(
            filename="path/to/somefile.py", lineno=3, text="something else wrong",
        )
